use poem::web::Data;
use poem_openapi::{param::Path, payload::Json, OpenApi, Object};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct Organization {
    pub id: String,
    pub name: String,
    pub tier: String,
    pub monthly_quota_usd: f64,
    pub current_usage_usd: f64,
    pub created_at: i64,
}

#[derive(Debug, Deserialize, Object)]
pub struct CreateOrganizationRequest {
    pub name: String,
    #[serde(default)]
    pub org_type: Option<String>, // "personal", "team", "company"
    #[serde(default)]
    pub industry: Option<String>,
}

pub struct OrganizationsApi {
    pub pool: PgPool,
}

#[OpenApi]
impl OrganizationsApi {
    /// Delete user's organization
    #[oai(path = "/organizations/:id", method = "delete", tag = "crate::api::ApiTags::Organizations")]
    async fn delete_organization(
        &self,
        #[oai(name = "id")] org_id: Path<String>,
        poem_req: &poem::Request,
    ) -> poem::Result<Json<String>> {
        // Extract user_id from session (injected by AuthMiddleware)
        let user = poem_req.extensions().get::<mawi_core::auth::User>()
            .ok_or_else(|| poem::Error::from_string("Authentication required", poem::http::StatusCode::UNAUTHORIZED))?;
        let user_id = &user.id;

        // Verify user owns this organization
        let user_org: Option<(Option<String>,)> = sqlx::query_as(
            "SELECT org_id FROM users WHERE id = $1"
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| poem::Error::from_string(e.to_string(), poem::http::StatusCode::INTERNAL_SERVER_ERROR))?;

        if let Some((Some(user_org_id),)) = user_org {
            if user_org_id != org_id.0 {
                return Err(poem::Error::from_string("Forbidden", poem::http::StatusCode::FORBIDDEN));
            }
        } else {
            return Err(poem::Error::from_string("Organization not found", poem::http::StatusCode::NOT_FOUND));
        }

        // Set user's org_id to NULL first (due to foreign key)
        sqlx::query("UPDATE users SET org_id = NULL WHERE org_id = $1")
            .bind(&org_id.0)
            .execute(&self.pool)
            .await
            .map_err(|e| poem::Error::from_string(e.to_string(), poem::http::StatusCode::INTERNAL_SERVER_ERROR))?;

        // Delete the organization
        sqlx::query("DELETE FROM organizations WHERE id = $1")
            .bind(&org_id.0)
            .execute(&self.pool)
            .await
            .map_err(|e| poem::Error::from_string(e.to_string(), poem::http::StatusCode::INTERNAL_SERVER_ERROR))?;

        Ok(Json("Organization deleted".to_string()))
    }

    /// Create a new organization
    #[oai(path = "/organizations", method = "post", tag = "crate::api::ApiTags::Organizations")]
    async fn create_organization(
        &self,
        req: Json<CreateOrganizationRequest>,
        poem_req: &poem::Request,
    ) -> poem::Result<Json<Organization>> {
        // Extract user_id from session (injected by AuthMiddleware)
        let user = poem_req.extensions().get::<mawi_core::auth::User>()
            .ok_or_else(|| poem::Error::from_string("Authentication required", poem::http::StatusCode::UNAUTHORIZED))?;
        let user_id = &user.id;

        let org_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp();

        // Create organization
        sqlx::query(
            "INSERT INTO organizations (id, name, tier, monthly_quota_usd, current_usage_usd, quota_reset_at, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
        )
        .bind(&org_id)
        .bind(&req.name)
        .bind("community") // Default tier
        .bind(0.0) // No quota for community
        .bind(0.0)
        .bind(now + 2592000) // 30 days from now
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| poem::Error::from_string(e.to_string(), poem::http::StatusCode::INTERNAL_SERVER_ERROR))?;

        // Link user to organization
        sqlx::query("UPDATE users SET org_id = $1 WHERE id = $2")
            .bind(&org_id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| poem::Error::from_string(e.to_string(), poem::http::StatusCode::INTERNAL_SERVER_ERROR))?;

        Ok(Json(Organization {
            id: org_id,
            name: req.name.clone(),
            tier: "community".to_string(),
            monthly_quota_usd: 0.0,
            current_usage_usd: 0.0,
            created_at: now,
        }))
    }
}
