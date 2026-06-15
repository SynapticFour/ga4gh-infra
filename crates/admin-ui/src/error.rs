use axum::http::StatusCode;
use axum::response::{IntoResponse as AxumIntoResponse, Response};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AdminUiError {
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("not found")]
    NotFound,
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("upstream unavailable: {0}")]
    Upstream(String),
    #[error("internal error: {0}")]
    Internal(String),
}

impl AdminUiError {
    pub fn status(&self) -> StatusCode {
        match self {
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Upstream(_) => StatusCode::SERVICE_UNAVAILABLE,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl AxumIntoResponse for AdminUiError {
    fn into_response(self) -> Response {
        let status = self.status();
        let body = self.to_string();
        (status, body).into_response()
    }
}

pub type AdminResult<T> = Result<T, AdminUiError>;
