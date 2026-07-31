use axum::{
    Json, Router,
    body::Body,
    http::{HeaderMap, Request, StatusCode, Uri},
    middleware::{self, Next},
    response::Response,
    routing::post,
};
use serde_json::{Value, json};

async fn rewrite_mcp_path(
    headers: HeaderMap,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // Because of .nest("/mcp", ...), req.uri().path() is relative to /mcp (e.g., "/" or "")
    let path = req.uri().path();

    if path == "/" || path.is_empty() {
        let method = headers.get("Mcp-Method").and_then(|v| v.to_str().ok());
        let name = headers.get("Mcp-Name").and_then(|v| v.to_str().ok());

        if let (Some(method), Some(name)) = (method, name) {
            let method = method.trim_matches('/');
            let name = name.trim_matches('/');

            // Construct relative path for the sub-router
            let new_path = format!("/{method}/{name}");

            let new_uri_str = match req.uri().query() {
                Some(query) => format!("{new_path}?{query}"),
                None => new_path,
            };

            if let Ok(new_uri) = new_uri_str.parse::<Uri>() {
                *req.uri_mut() = new_uri;
            } else {
                return Err(StatusCode::BAD_REQUEST);
            }
        } else {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    Ok(next.run(req).await)
}

pub fn router() -> Router {
    Router::new()
        .route("/tools/call/echo", post(echo))
        .route("/tools/list", post(list_tools))
        .layer(middleware::from_fn(rewrite_mcp_path))
}

pub async fn list_tools(Json(_params): Json<Value>) -> Json<Value> {
    Json(json!({"result": "ok"}))
}

pub async fn echo(Json(_params): Json<Value>) -> Json<Value> {
    Json(json!({"result": "ok"}))
}
