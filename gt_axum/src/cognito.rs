use axum::extract::FromRequestParts;
use axum::http::StatusCode;
use lambda_http::RequestExt;

/// Extracts Cognito User ID from the request.
#[derive(Debug, Clone)]
pub struct CognitoUserId(pub String);

impl<S> FromRequestParts<S> for CognitoUserId
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        tracing::info!("Extracting Cognito user ID");

        // In debug mode, allow a fixed user ID for testing purposes
        #[cfg(debug_assertions)]
        {
            return Ok(Self(
                "b8d11300-d0d1-70aa-2665-6d2a9535ffcc".to_string(),
            ));
        }

        #[cfg(not(debug_assertions))]
        parts
            .request_context_ref()
            .and_then(|ctx| ctx.authorizer())
            .and_then(|auth| {
                auth.jwt
                    .as_ref()
                    .map(|jwt| &jwt.claims)
                    .and_then(|claims| claims.get("sub"))
            })
            .map_or(
                Err((StatusCode::UNAUTHORIZED, "Unauthorized")),
                |cognito_user_id| Ok(Self(cognito_user_id.to_string())),
            )
    }
}

/// Extracts optional Cognito User ID from the request (does not fail if not authenticated).
#[derive(Debug, Clone)]
pub struct OptionalCognitoUserId(pub Option<String>);

impl<S> FromRequestParts<S> for OptionalCognitoUserId
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        tracing::info!("Extracting optional Cognito user ID");
        let user_id = parts
            .request_context_ref()
            .and_then(|ctx| ctx.authorizer())
            .and_then(|auth| {
                auth.jwt
                    .as_ref()
                    .map(|jwt| &jwt.claims)
                    .and_then(|claims| claims.get("sub"))
            })
            .map(|cognito_user_id| cognito_user_id.to_string());
        Ok(Self(user_id))
    }
}
