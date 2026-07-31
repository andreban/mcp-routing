mod mcp;

use std::error::Error;

use axum::Router;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let app = Router::new().nest("/mcp", mcp::router());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    axum::serve(listener, app).await?;

    Ok(())
}
