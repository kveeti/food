use axum::Router;
use tower_http::services::{ServeDir, ServeFile};

pub fn router(dir: &str) -> Router {
    let index = format!("{dir}/index.html");
    Router::new().fallback_service(ServeDir::new(dir).fallback(ServeFile::new(index)))
}
