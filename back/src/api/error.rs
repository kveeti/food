use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("{0}")]
    BadRequest(String),
    #[error(transparent)]
    Db(#[from] sqlx::Error),
    #[error("settings required")]
    SettingsRequired,
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        tracing::error!(error = ?self, "api error");
        let status = match &self {
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::SettingsRequired => StatusCode::CONFLICT,
            ApiError::Db(_) | ApiError::Anyhow(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let message = match &self {
            ApiError::BadRequest(message) => message.clone(),
            ApiError::SettingsRequired => "settings_required".to_string(),
            ApiError::Db(_) | ApiError::Anyhow(_) => "internal server error".to_string(),
        };

        (status, Json(ErrorBody { error: message })).into_response()
    }
}
