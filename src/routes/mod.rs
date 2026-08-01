use axum::Router;
use tower_http::trace::TraceLayer;

mod mcp;

pub fn router() -> Router {
    Router::new()
        .nest("/mcp", mcp::router())
        .layer(TraceLayer::new_for_http())
}
