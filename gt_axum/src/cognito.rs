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
