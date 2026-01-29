use anyhow::{anyhow, Result};
use sqlx::PgPool;


// ============================================================================
// Quota Management
// ============================================================================

pub struct QuotaManager {
    db: PgPool,
}

impl QuotaManager {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    // ------------------------------------------------------------------------
    // Quota Checking
    // ------------------------------------------------------------------------

    /// Check if user has enough quota for the estimated cost
    ///  
    /// Hybrid model: Check personal quota first, then org quota
    pub async fn check_quota(&self, user_id: &str, estimated_cost_usd: f64) -> Result<bool> {
        let user: crate::auth::User = sqlx::query_as("SELECT * FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&self.db)
            .await?;

        // Check personal quota first
        // RELAXATION: If user is free tier, allow if below a reasonable cap (e.g. $10) or just Bypass
        // The user requested "Quota isn't really needed for Community".
        if user.is_free_tier {
            return Ok(true);
        }

        let personal_remaining = user.monthly_quota_usd - user.current_usage_usd;
        if personal_remaining >= estimated_cost_usd {
            return Ok(true);
        }

        // If user has an org, check org quota
        if let Some(org_id) = &user.org_id {
            let org: Organization = sqlx::query_as("SELECT * FROM organizations WHERE id = $1")
                .bind(org_id)
                .fetch_one(&self.db)
                .await?;

            let org_remaining = org.monthly_quota_usd - org.current_usage_usd;
            if org_remaining >= estimated_cost_usd {
                return Ok(true);
            }
        }

        Ok(false)
    }

    // ------------------------------------------------------------------------
    // Quota Charging
    // ------------------------------------------------------------------------

    /// Charge user for actual cost
    ///
    /// Hybrid model: Charge personal quota first, overflow to org
    pub async fn charge_user(&self, user_id: &str, actual_cost_usd: f64) -> Result<()> {
        if actual_cost_usd <= 0.0 {
            return Ok(());
        }

        let user: crate::auth::User = sqlx::query_as("SELECT * FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&self.db)
            .await?;

        let personal_remaining = user.monthly_quota_usd - user.current_usage_usd;

        if personal_remaining >= actual_cost_usd {
            // Charge fully from personal quota
            self.charge_personal_quota(user_id, actual_cost_usd).await?;
        } else if let Some(org_id) = &user.org_id {
            // Split charge: personal + org
            if personal_remaining > 0.0 {
                self.charge_personal_quota(user_id, personal_remaining).await?;
            }

            let org_charge = actual_cost_usd - personal_remaining;
            self.charge_org_quota(org_id, org_charge).await?;
        } else {
            return Err(anyhow!("Insufficient quota"));
        }

        Ok(())
    }

    async fn charge_personal_quota(&self, user_id: &str, amount: f64) -> Result<()> {
        let updated = sqlx::query(
            "UPDATE users 
             SET current_usage_usd = current_usage_usd + $1, 
                 updated_at = $2 
             WHERE id = $3 
               AND current_usage_usd + $4 <= monthly_quota_usd",
        )
        .bind(amount)
        .bind(chrono::Utc::now().timestamp())
        .bind(user_id)
        .bind(amount)
        .execute(&self.db)
        .await?;

        if updated.rows_affected() == 0 {
            return Err(anyhow!("Insufficient personal quota"));
        }

        Ok(())
    }

    async fn charge_org_quota(&self, org_id: &str, amount: f64) -> Result<()> {
        let updated = sqlx::query(
            "UPDATE organizations 
             SET current_usage_usd = current_usage_usd + $1, 
                 updated_at = $2 
             WHERE id = $3 
               AND current_usage_usd + $4 <= monthly_quota_usd",
        )
        .bind(amount)
        .bind(chrono::Utc::now().timestamp())
        .bind(org_id)
        .bind(amount)
        .execute(&self.db)
        .await?;

        if updated.rows_affected() == 0 {
            return Err(anyhow!("Insufficient organization quota"));
        }

        Ok(())
    }

    // ------------------------------------------------------------------------
    // Quota Queries
    // ------------------------------------------------------------------------

    /// Get user's quota status
    pub async fn get_user_quota_status(&self, user_id: &str) -> Result<QuotaStatus> {
        let user: crate::auth::User = sqlx::query_as("SELECT * FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&self.db)
            .await?;

        let personal_remaining = user.monthly_quota_usd - user.current_usage_usd;
        let personal_percentage = crate::utils::safe_percentage(user.current_usage_usd, user.monthly_quota_usd);

        let mut org_quota_available = 0.0;
        let mut org_quota_percentage = 0;

        if let Some(org_id) = &user.org_id {
            if let Ok(org) = sqlx::query_as::<_, Organization>("SELECT * FROM organizations WHERE id = $1")
                .bind(org_id)
                .fetch_one(&self.db)
                .await
            {
                org_quota_available = org.monthly_quota_usd - org.current_usage_usd;
                org_quota_percentage = crate::utils::safe_percentage(org.current_usage_usd, org.monthly_quota_usd);
            }
        }

        Ok(QuotaStatus {
            personal_quota: user.monthly_quota_usd,
            personal_used: user.current_usage_usd,
            personal_remaining,
            personal_percentage,
            org_quota_available,
            org_percentage: org_quota_percentage,
            total_available: personal_remaining + org_quota_available,
        })
    }

    // ------------------------------------------------------------------------
    // Quota Resets (called by cron job)
    // ------------------------------------------------------------------------

    /// Reset quotas for users whose reset_at timestamp has passed
    pub async fn reset_expired_quotas(&self) -> Result<usize> {
        let now = chrono::Utc::now().timestamp();

        // Reset user quotas
        let user_result = sqlx::query(
            "UPDATE users 
             SET current_usage_usd = 0.0, 
                 quota_reset_at = $1, 
                 updated_at = $2 
             WHERE quota_reset_at <= $3",
        )
        .bind(Self::next_month_timestamp())
        .bind(now)
        .bind(now)
        .execute(&self.db)
        .await?;

        // Reset org quotas
        let org_result = sqlx::query(
            "UPDATE organizations 
             SET current_usage_usd = 0.0, 
                 quota_reset_at = $1, 
                 updated_at = $2 
             WHERE quota_reset_at <= $3",
        )
        .bind(Self::next_month_timestamp())
        .bind(now)
        .bind(now)
        .execute(&self.db)
        .await?;

        Ok((user_result.rows_affected() + org_result.rows_affected()) as usize)
    }

    fn next_month_timestamp() -> i64 {
        crate::utils::next_month_timestamp()
    }
}

// ============================================================================
// Types
// ============================================================================

use serde::Serialize;
use sqlx::FromRow;

#[derive(Debug, Serialize)]
pub struct QuotaStatus {
    pub personal_quota: f64,
    pub personal_used: f64,
    pub personal_remaining: f64,
    pub personal_percentage: u8, // 0-100
    pub org_quota_available: f64,
    pub org_percentage: u8,
    pub total_available: f64,
}

#[derive(Debug, FromRow)]
struct Organization {
    #[allow(dead_code)]
    id: String,
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    tier: String,
    monthly_quota_usd: f64,
    current_usage_usd: f64,
    #[allow(dead_code)]
    quota_reset_at: i64,
    #[allow(dead_code)]
    created_at: i64,
    #[allow(dead_code)]
    updated_at: i64,
}
