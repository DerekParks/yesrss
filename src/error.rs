use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("not found")]
    NotFound,
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    Conflict(String),
    #[error("too many attempts, try again later")]
    TooManyRequests,
    #[error(transparent)]
    Db(sqlx::Error),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    pub fn bad(msg: impl Into<String>) -> Self {
        Self::BadRequest(msg.into())
    }

    pub fn status(&self) -> StatusCode {
        match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::TooManyRequests => StatusCode::TOO_MANY_REQUESTS,
            Self::Db(_) | Self::Other(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn public_message(&self) -> String {
        match self {
            Self::Db(_) | Self::Other(_) => "internal error".into(),
            other => other.to_string(),
        }
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        match e {
            sqlx::Error::RowNotFound => Self::NotFound,
            sqlx::Error::Database(ref d) if d.is_unique_violation() => {
                Self::Conflict("already exists".into())
            }
            e => Self::Db(e),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status();
        if status.is_server_error() {
            tracing::error!(error = ?self, "request failed");
        }
        (status, Json(json!({ "error": self.public_message() }))).into_response()
    }
}
