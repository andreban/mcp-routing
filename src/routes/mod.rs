use axum::Router;

mod mcp;

pub fn router() -> Router {
    Router::new().nest("/mcp", mcp::router())
}
