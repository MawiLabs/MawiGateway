use poem::{error::Error, http::StatusCode, Endpoint, Middleware, Request, Result};

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
        let pool = req.data::<sqlx::PgPool>().ok_or_else(|| {
            Error::from_string(
                "Database connection not available",
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;

        // 2. Validate Authentication. Returns (User, scopes) where
        //    scopes is the API key's scope list (["admin"] for
        //    session-cookie auth). Both flow into request extensions
        //    so handlers can either check identity (User) or
        //    authorization (AuthScopes).
        match super::utils::get_current_user_and_scopes(&req, pool).await {
            Ok((user, scopes)) => {
                req.extensions_mut().insert(user);
                req.extensions_mut()
                    .insert(super::utils::AuthScopes(scopes));

                // PROCEED.
                self.ep.call(req).await
            }
            Err(e) => Err(e),
        }
    }
}
