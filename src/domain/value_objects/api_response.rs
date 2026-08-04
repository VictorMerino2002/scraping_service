use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response as AxumResponse};
use axum::Json;
use serde::Serialize;
use serde_json::json;

use crate::domain::value_objects::Error;

pub struct ApiResponse<T> {
    pub result: Result<T, Error>,
    pub code: StatusCode,
    pub success_on_error: bool,
    pub redirect_to: Option<String>,
    pub headers: Option<HeaderMap>,
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> AxumResponse {
        if let Some(url) = self.redirect_to {
            let headers = self.headers.unwrap_or_default();
            let redirect = Redirect::to(&url);
            return (StatusCode::FOUND, headers, redirect).into_response();
        }

        match self.result {
            Ok(payload) => (
                self.code,
                Json(json!({
                    "success": true,
                    "payload": payload
                })),
            )
                .into_response(),

            Err(error) => {
                let status = if self.success_on_error {
                    StatusCode::OK
                } else {
                    status_code_for(&error)
                };

                (
                    status,
                    Json(json!({
                        "success": false,
                        "message": error.to_string()
                    })),
                )
                    .into_response()
            }
        }
    }
}

impl<T> ApiResponse<T> {
    pub fn default(result: Result<T, Error>) -> Self {
        Self {
            result,
            code: StatusCode::OK,
            success_on_error: false,
            redirect_to: None,
            headers: None,
        }
    }

    pub fn with_code(mut self, code: StatusCode) -> Self {
        self.code = code;
        self
    }

    pub fn success_on_error(mut self, value: bool) -> Self {
        self.success_on_error = value;
        self
    }

    pub fn with_redirect(mut self, url: impl Into<String>) -> Self {
        self.redirect_to = Some(url.into());
        self
    }

    pub fn with_headers(mut self, headers: HeaderMap) -> Self {
        self.headers = Some(headers);
        self
    }
}

fn status_code_for(error: &Error) -> StatusCode {
    match error {
        Error::NotFound(_) => StatusCode::NOT_FOUND,
        Error::InvalidInput(_) => StatusCode::BAD_REQUEST,
        Error::Unauthorized(_) => StatusCode::UNAUTHORIZED,
        Error::NetworkError(_) => StatusCode::BAD_GATEWAY,
        Error::DatabaseError(_) | Error::Unknown(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
