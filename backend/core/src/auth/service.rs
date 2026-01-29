use anyhow::{anyhow, Result};
use bcrypt::{hash, verify, DEFAULT_COST};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: Option<String>,
    pub name: Option<String>,
    pub org_id: Option<String>,
    pub tier: String, // 'A', 'B', 'C'
    pub monthly_quota_usd: f64,
    pub current_usage_usd: f64,
    pub quota_reset_at: i64,
    pub is_admin: bool,
    pub is_free_tier: bool,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_login_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub session_token: String,
    pub user: UserProfile,
}

#[derive(Debug, Serialize)]
pub struct UserProfile {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
    pub tier: String,
    pub monthly_quota_usd: f64,
    pub current_usage_usd: f64,
    pub quota_remaining_usd: f64,
    pub is_free_tier: bool,
    pub org_id: Option<String>,
}

#[derive(Debug, Clone, FromRow)]
pub struct Session {
    pub id: String,
    pub user_id: String,
    pub expires_at: i64,
    pub created_at: i64,
}

// ============================================================================
// Service
// ============================================================================

pub struct AuthService {
    db: PgPool,
}

impl AuthService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    // ------------------------------------------------------------------------
    // Registration
    // ------------------------------------------------------------------------

    pub async fn register(&self, req: RegisterRequest) -> Result<AuthResponse> {
        // Validate email
        if !Self::is_valid_email(&req.email) {
            return Err(anyhow!("Invalid email address"));
        }

        // Check if email already exists
        let existing: Option<User> = sqlx::query_as("SELECT * FROM users WHERE email = $1")
            .bind(&req.email)
            .fetch_optional(&self.db)
            .await?;

        if existing.is_some() {
            return Err(anyhow!("Email already registered"));
        }

        // Hash password
        let password_hash = hash(&req.password, DEFAULT_COST)?;

        // Create user
        let user_id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp();
        let quota_reset_at = Self::next_month_timestamp();

        // Check if this is the first user (First User is Admin policy)
        let user_count: i64 = sqlx::query_scalar("SELECT count(*) FROM users")
            .fetch_one(&self.db)
            .await
            .unwrap_or(0);

        let (tier, monthly_quota, is_free, is_admin) = if user_count == 0 {
            // First user becomes System Admin (Enterprise Tier)
            ("enterprise".to_string(), 999999.0, false, true)
        } else {
            // Subsequent users get default free tier
            let default_plan = crate::plans::get_default_plan();
            (default_plan.id, default_plan.monthly_quota_usd, default_plan.is_free, false)
        };

        sqlx::query(
            "INSERT INTO users (
                id, email, password_hash, name, tier, 
                monthly_quota_usd, current_usage_usd, quota_reset_at,
                is_admin, is_free_tier, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        )
        .bind(&user_id)
        .bind(&req.email)
        .bind(&password_hash)
        .bind(&req.name)
        .bind(tier)
        .bind(monthly_quota)
        .bind(0.0) // current_usage
        .bind(quota_reset_at)
        .bind(is_admin)
        .bind(is_free)
        .bind(now)
        .bind(now)
        .execute(&self.db)
        .await?;

        // Fetch created user
        let user: User = sqlx::query_as("SELECT * FROM users WHERE id = $1")
            .bind(&user_id)
            .fetch_one(&self.db)
            .await?;

        // Create session
        let session_token = self.create_session(&user_id).await?;

        Ok(AuthResponse {
            session_token,
            user: Self::to_profile(&user),
        })
    }

    // ------------------------------------------------------------------------
    // Login
    // ------------------------------------------------------------------------

    pub async fn login(&self, req: LoginRequest) -> Result<AuthResponse> {
        // Find user by email
        let user: User = sqlx::query_as("SELECT * FROM users WHERE email = $1")
            .bind(&req.email)
            .fetch_optional(&self.db)
            .await?
            .ok_or_else(|| anyhow!("Invalid email or password"))?;

        // Verify password
        let password_hash = user
            .password_hash
            .as_ref()
            .ok_or_else(|| anyhow!("Invalid email or password"))?;

        // Sanitize verification error (e.g. invalid hash format)
        match verify(&req.password, password_hash) {
            Ok(true) => {}, // Password valid
            Ok(false) | Err(_) => {
                // Log the real error internally if needed?
                return Err(anyhow!("Invalid email or password"));
            }
        }

        // Update last login
        let now = chrono::Utc::now().timestamp();
        sqlx::query("UPDATE users SET last_login_at = $1 WHERE id = $2")
            .bind(now)
            .bind(&user.id)
            .execute(&self.db)
            .await?;

        // Create session
        let session_token = self.create_session(&user.id).await?;

        Ok(AuthResponse {
            session_token,
            user: Self::to_profile(&user),
        })
    }

    // ------------------------------------------------------------------------
    // Logout
    // ------------------------------------------------------------------------

    pub async fn logout(&self, session_token: &str) -> Result<()> {
        sqlx::query("DELETE FROM sessions WHERE id = $1")
            .bind(session_token)
            .execute(&self.db)
            .await?;

        Ok(())
    }

    // ------------------------------------------------------------------------
    // Session Management
    // ------------------------------------------------------------------------

    async fn create_session(&self, user_id: &str) -> Result<String> {
        let session_id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp();
        let expires_at = now + (30 * 24 * 60 * 60); // 30 days

        sqlx::query(
            "INSERT INTO sessions (id, user_id, expires_at, created_at) VALUES ($1, $2, $3, $4)",
        )
        .bind(&session_id)
        .bind(user_id)
        .bind(expires_at)
        .bind(now)
        .execute(&self.db)
        .await?;

        Ok(session_id)
    }

    pub async fn validate_session(&self, session_token: &str) -> Result<User> {
        // Fetch session
        let session: Session = sqlx::query_as("SELECT * FROM sessions WHERE id = $1")
            .bind(session_token)
            .fetch_optional(&self.db)
            .await?
            .ok_or_else(|| anyhow!("Invalid session"))?;

        // Check expiry
        let now = chrono::Utc::now().timestamp();
        if session.expires_at < now {
            // Delete expired session
            sqlx::query("DELETE FROM sessions WHERE id = $1")
                .bind(session_token)
                .execute(&self.db)
                .await?;

            return Err(anyhow!("Session expired"));
        }


        // Extend session (Sliding Window)
        let new_expiry = now + (30 * 24 * 60 * 60); // 30 days
        sqlx::query("UPDATE sessions SET expires_at = $1 WHERE id = $2")
            .bind(new_expiry)
            .bind(session_token)
            .execute(&self.db)
            .await?;

        // Fetch user
        let user: User = sqlx::query_as("SELECT * FROM users WHERE id = $1")
            .bind(&session.user_id)
            .fetch_one(&self.db)
            .await?;

        Ok(user)
    }

    // ------------------------------------------------------------------------
    // User Queries
    // ------------------------------------------------------------------------

    pub async fn get_user_by_id(&self, user_id: &str) -> Result<User> {
        let user: User = sqlx::query_as("SELECT * FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&self.db)
            .await?
            .ok_or_else(|| anyhow!("User not found"))?;

        Ok(user)
    }

    // ------------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------------

    fn is_valid_email(email: &str) -> bool {
        // Basic email validation: 
        // - Must have exactly one @
        // - Local part (before @) must be non-empty and alphanumeric with allowed chars
        // - Domain part (after @) must have at least one dot and valid chars
        if email.len() < 5 || email.len() > 254 {
            return false;
        }
        
        let parts: Vec<&str> = email.split('@').collect();
        if parts.len() != 2 {
            return false;
        }
        
        let local = parts[0];
        let domain = parts[1];
        
        // Local part: non-empty, valid characters
        if local.is_empty() || local.len() > 64 {
            return false;
        }
        
        // Domain must have at least one dot and valid structure
        if !domain.contains('.') || domain.starts_with('.') || domain.ends_with('.') {
            return false;
        }
        
        // Check for valid domain characters
        let domain_valid = domain.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-');
        if !domain_valid {
            return false;
        }
        
        // Local part should have valid characters (alphanumeric + ._-+)
        let local_valid = local.chars().all(|c| c.is_alphanumeric() || "._-+".contains(c));
        
        local_valid
    }

    fn next_month_timestamp() -> i64 {
        crate::utils::next_month_timestamp()
    }

    pub fn to_profile(user: &User) -> UserProfile {
        UserProfile {
            id: user.id.clone(),
            email: user.email.clone(),
            name: user.name.clone(),
            tier: user.tier.clone(),
            monthly_quota_usd: user.monthly_quota_usd,
            current_usage_usd: user.current_usage_usd,
            quota_remaining_usd: user.monthly_quota_usd - user.current_usage_usd,
            is_free_tier: user.is_free_tier,
            org_id: user.org_id.clone(),
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_next_month_timestamp() {
        let ts = AuthService::next_month_timestamp();
        let now = chrono::Utc::now().timestamp();
        assert!(ts > now);
    }
}
