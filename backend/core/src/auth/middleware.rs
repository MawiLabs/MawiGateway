use poem::{Endpoint, Middleware, Request, Result, error::Error, http::StatusCode};


pub struct AuthMiddleware;

impl<E: Endpoint> Middleware<E> for AuthMiddleware {
    type Output = AuthMiddlewareEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        AuthMiddlewareEndpoint { ep }
    }
}

pub struct AuthMiddlewareEndpoint<E> {
    ep: E,
}

impl<E: Endpoint> Endpoint for AuthMiddlewareEndpoint<E> {
    type Output = E::Output;

    async fn call(&self, mut req: Request) -> Result<Self::Output> {
        // Skip auth check for login/register endpoints
        let path = req.uri().path();
        if path == "/v1/auth/login" || path == "/v1/auth/register" || path == "/v1/auth/logout" {
            return self.ep.call(req).await;
        }
        
        // 1. Get DB pool (needed for auth check)
        let pool = req.data::<sqlx::PgPool>()
            .ok_or_else(|| Error::from_string("Database connection not available", StatusCode::INTERNAL_SERVER_ERROR))?;

        // 2. Validate Authentication (supports both API Keys and Session Cookies via shared utility)
        // 2. Validate Authentication (supports both API Keys and Session Cookies via shared utility)
        match super::utils::get_current_user(&req, pool).await {
            Ok(user) => {
                // Attach User object to request extensions so handlers can access it
                req.extensions_mut().insert(user);
                
                // PROCEED.
                self.ep.call(req).await
            }
            Err(e) => {
                // Propagate the specific error (e.g. "Missing session token", "API Key expired", etc)
                Err(e)
            }
        }
    }
}
