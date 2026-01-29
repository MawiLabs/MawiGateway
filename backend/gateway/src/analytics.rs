use poem_openapi::{payload::Json, OpenApi};
use sqlx::PgPool;
use poem_openapi::Object;
use serde::{Serialize, Deserialize};
use crate::api::ApiTags;
use poem::Request;
use mawi_core::auth::User;

#[derive(Debug, Serialize, Deserialize, Object, sqlx::FromRow)]
pub struct RequestLog {
    pub id: String,
    pub service_name: String,
    pub model_id: String,
    pub provider_type: String,
    pub latency_ms: i64,
    pub status: String,
    pub created_at: String,
    pub tokens_prompt: Option<i64>,
    pub tokens_completion: Option<i64>,
    pub tokens_total: Option<i64>,
    pub cost_usd: Option<f64>,
    pub error_message: Option<String>,
    pub failover_count: i64,
}

#[derive(Debug, Serialize, Deserialize, Object, sqlx::FromRow)]
pub struct AnalyticsSummary {
    pub total_requests: i64,
    pub successful_requests: i64,
    pub failed_requests: i64,
    pub total_cost_usd: f64,
    pub total_tokens: i64,
    pub avg_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
}

#[derive(Debug, Serialize, Deserialize, Object, sqlx::FromRow)]
pub struct TimeSeriesPoint {
    pub timestamp: String,
    pub request_count: i64,
    pub error_count: i64,
    pub avg_latency_ms: f64,
    pub total_cost_usd: f64,
    pub total_tokens: i64,
}

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct TopModel {
    pub model_id: String,
    pub model_name: String,
    pub request_count: i64,
    pub total_cost: f64,
}

pub struct AnalyticsApi {
    pub pool: PgPool,
}

#[derive(Debug, sqlx::FromRow)]
struct RequestLogRow {
    id: String,
    service_name: String,
    model_id: String,
    provider_type: String,
    latency_ms: i64,
    status: String,
    created_at: i64,
    tokens_prompt: Option<i64>,
    tokens_completion: Option<i64>,
    tokens_total: Option<i64>,
    cost_usd: Option<f64>,
    error_message: Option<String>,
    failover_count: i64,
}

#[OpenApi]
impl AnalyticsApi {
    /// Get analytics overview with summary, time-series, and top models
    #[oai(path = "/analytics/summary", method = "get", tag = "ApiTags::Analytics")]
    async fn get_summary(&self, req: &Request) -> poem::Result<Json<AnalyticsSummary>> {
        let user = req.extensions().get::<User>().ok_or_else(|| {
             poem::error::Error::from_string("Unauthorized", poem::http::StatusCode::UNAUTHORIZED)
        })?;

        // Advanced Aggregation
        #[derive(sqlx::FromRow)]
        struct StatsRow {
            total_requests: i64,
            successful_requests: i64,
            failed_requests: i64,
            total_cost_usd: f64,
            total_tokens: i64,
            avg_latency_ms: f64,
        }

        let stats = sqlx::query_as::<_, StatsRow>(
            r#"
            SELECT 
                COUNT(*) as total_requests,
                COUNT(CASE WHEN status = 'success' THEN 1 END) as successful_requests,
                COUNT(CASE WHEN status = 'error' THEN 1 END) as failed_requests,
                COALESCE(SUM(cost_usd), 0.0)::FLOAT8 as total_cost_usd,
                COALESCE(SUM(tokens_total), 0) as total_tokens,
                COALESCE(AVG(latency_ms), 0.0)::FLOAT8 as avg_latency_ms
            FROM request_logs
            WHERE user_id = $1
            "#
        )
        .bind(&user.id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e: sqlx::Error| poem::error::Error::from_string(e.to_string(), poem::http::StatusCode::INTERNAL_SERVER_ERROR))?;

        // Calculate Percentiles
        #[derive(sqlx::FromRow)]
        struct LatencyRow {
            latency_ms: Option<i64>,
        }

        let latencies = sqlx::query_as::<_, LatencyRow>(
            "SELECT latency_ms FROM request_logs WHERE status = 'success' AND user_id = $1 ORDER BY latency_ms ASC"
        )
        .bind(&user.id)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        let count = latencies.len();
        let p95_latency = if count > 0 {
            let idx = (count as f64 * 0.95) as usize;
            latencies.get(idx).map(|r| r.latency_ms.unwrap_or(0) as f64).unwrap_or(0.0)
        } else {
            0.0
        };
        
        let p99_latency = if count > 0 {
            let idx = (count as f64 * 0.99) as usize;
            latencies.get(idx).map(|r| r.latency_ms.unwrap_or(0) as f64).unwrap_or(0.0)
        } else {
            0.0
        };

        Ok(Json(AnalyticsSummary {
            total_requests: stats.total_requests,
            successful_requests: stats.successful_requests,
            failed_requests: stats.failed_requests,
            total_cost_usd: stats.total_cost_usd,
            total_tokens: stats.total_tokens,
            avg_latency_ms: stats.avg_latency_ms,
            p95_latency_ms: p95_latency,
            p99_latency_ms: p99_latency,
        }))
    }

    /// Get time-series data for charts
    #[oai(path = "/analytics/time-series", method = "get", tag = "ApiTags::Analytics")]
    async fn get_time_series(
        &self,
        req: &Request,
        #[oai(name = "range")] range: poem_openapi::param::Query<Option<String>>, // 24h, 7d, 30d
    ) -> poem::Result<Json<Vec<TimeSeriesPoint>>> {
        let user = req.extensions().get::<User>().ok_or_else(|| {
             poem::error::Error::from_string("Unauthorized", poem::http::StatusCode::UNAUTHORIZED)
        })?;

        // Group by Hour (for 24h) or Day (for longer)
        // Default to last 24h hourly
        let range_val = range.0.unwrap_or_else(|| "24h".to_string());
        
        let (group_format, interval_str) = match range_val.as_str() {
            "7d" => ("YYYY-MM-DD", "7 days"),
            "30d" => ("YYYY-MM-DD", "30 days"),
            _ => ("YYYY-MM-DD HH24:00:00", "24 hours"),
        };

        let rows = sqlx::query_as::<_, TimeSeriesPoint>(
            &format!(r#"
            SELECT 
                to_char(to_timestamp(created_at), '{}') as timestamp,
                COUNT(*)::BIGINT as request_count,
                COUNT(CASE WHEN status = 'error' THEN 1 END)::BIGINT as error_count,
                COALESCE(AVG(latency_ms), 0.0)::FLOAT8 as avg_latency_ms,
                COALESCE(SUM(cost_usd), 0.0)::FLOAT8 as total_cost_usd,
                COALESCE(SUM(tokens_total), 0)::BIGINT as total_tokens
            FROM request_logs
            WHERE created_at >= EXTRACT(EPOCH FROM (NOW() - INTERVAL '{}'))::BIGINT
            AND user_id = $1
            GROUP BY timestamp
            ORDER BY timestamp ASC
            "#, group_format, interval_str)
        )
        .bind(&user.id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e: sqlx::Error| poem::error::Error::from_string(e.to_string(), poem::http::StatusCode::INTERNAL_SERVER_ERROR))?;

        Ok(Json(rows))
    }
    
    /// Get top models with cost analysis
    #[oai(path = "/analytics/top-models", method = "get", tag = "ApiTags::Analytics")]
    async fn get_top_models(&self, req: &Request) -> poem::Result<Json<Vec<TopModel>>> {
        let user = req.extensions().get::<User>().ok_or_else(|| {
             poem::error::Error::from_string("Unauthorized", poem::http::StatusCode::UNAUTHORIZED)
        })?;

        let models = sqlx::query_as::<_, (String, String, i64, f64)>(
            "SELECT 
                m.id as model_id,
                m.name as model_name,
                COUNT(rl.id) as request_count,
                COALESCE(SUM(rl.cost_usd), 0.0)::FLOAT8 as total_cost
             FROM models m
             LEFT JOIN request_logs rl ON m.id = rl.model_id
             WHERE rl.user_id = $1
             GROUP BY m.id, m.name
             HAVING COUNT(rl.id) > 0
             ORDER BY total_cost DESC
             LIMIT 10"
        )
        .bind(&user.id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e: sqlx::Error| poem::error::Error::from_string(e.to_string(), poem::http::StatusCode::INTERNAL_SERVER_ERROR))?;
        
        let top_models: Vec<TopModel> = models.into_iter().map(|(model_id, model_name, request_count, total_cost)| {
            TopModel { model_id, model_name, request_count, total_cost }
        }).collect();
        
        Ok(Json(top_models))
    }

    /// Legacy getter for simple lists (paginated)
    #[oai(path = "/analytics/requests", method = "get", tag = "ApiTags::Analytics")]
    async fn get_requests(
        &self,
        req: &Request,
        #[oai(name = "limit")] limit: poem_openapi::param::Query<Option<i64>>,
    ) -> poem::Result<Json<Vec<RequestLog>>> {
        let user = req.extensions().get::<User>().ok_or_else(|| {
             poem::error::Error::from_string("Unauthorized", poem::http::StatusCode::UNAUTHORIZED)
        })?;

        let limit_val = limit.0.unwrap_or(50).min(500);
        
        let rows = sqlx::query_as::<_, RequestLogRow>(
            "SELECT id, service_name, model_id, provider_type, latency_ms, status, created_at, 
                    tokens_prompt, tokens_completion, tokens_total, cost_usd, error_message, failover_count
             FROM request_logs 
             WHERE user_id = $1
             ORDER BY created_at DESC 
             LIMIT $2"
        )
        .bind(&user.id)
        .bind(limit_val)
        .fetch_all(&self.pool)
        .await
        .map_err(|e: sqlx::Error| poem::error::Error::from_string(e.to_string(), poem::http::StatusCode::INTERNAL_SERVER_ERROR))?;

        // Convert timestamps
        let logs: Vec<RequestLog> = rows.into_iter().map(|row| {
            RequestLog {
                id: row.id,
                service_name: row.service_name,
                model_id: row.model_id,
                provider_type: row.provider_type,
                latency_ms: row.latency_ms,
                status: row.status,
                created_at: row.created_at.to_string(),
                tokens_prompt: row.tokens_prompt,
                tokens_completion: row.tokens_completion,
                tokens_total: row.tokens_total,
                cost_usd: row.cost_usd,
                error_message: row.error_message,
                failover_count: row.failover_count,
            }
        }).collect();

        Ok(Json(logs))
    }
}
