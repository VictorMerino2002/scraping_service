use std::sync::Arc;

use axum::{extract::FromRequestParts, http::request::Parts};

use crate::{
    domain::value_objects::{ApiResponse, Error},
    infrastructure::bootstrap::AppState,
};

pub struct AuthHeader;

impl FromRequestParts<Arc<AppState>> for AuthHeader {
    type Rejection = ApiResponse<()>;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let secret = parts
            .headers
            .get("x-auth")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .ok_or_else(|| {
                ApiResponse::default(Err(Error::Unauthorized("missing secret key".to_string())))
            })?;

        let expected_secret =
            std::env::var("SECRET_KEY").expect("SECRET_KEY must be set in environment variables");
        if secret != expected_secret {
            return Err(ApiResponse::default(Err(Error::Unauthorized(
                "invalid secret key".to_string(),
            ))));
        }

        Ok(AuthHeader)
    }
}
