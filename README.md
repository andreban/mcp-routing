# mcp-routing

A [Tower](https://crates.io/crates/tower)-native routing library for building [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) servers in Rust.

> **Note:** `mcp-routing` exclusively supports the **stateless** version of the Model Context Protocol ([`2026-07-28`](https://modelcontextprotocol.io/docs/2026-07-28/)). It uses request-based discovery (`server/discover`) and direct tool execution, and does **not** support previous stateful protocol versions (e.g. 2024-11-05 `initialize` lifecycle).

`mcp-routing` provides a composable, framework-agnostic [`McpRouter`] that implements [`tower::Service`]. It can be plugged directly into [Axum](https://crates.io/crates/axum), [Hyper](https://crates.io/crates/hyper), or any custom Tower middleware pipeline.

## Features

- **Stateless Protocol**: Built specifically for the [2026-07-28 MCP specification](https://modelcontextprotocol.io/docs/2026-07-28/), featuring `server/discover` and stateless HTTP request routing.
- **Tower-Native**: Implements `tower::Service` for any HTTP request body implementing `http_body::Body`.
- **Header-Based Routing**: Dispatches requests via standard `Mcp-Method` and `Mcp-Name` headers per the MCP HTTP spec.
- **Typed Tool Handlers**: Register async Rust functions as MCP tools with automatic JSON-RPC argument deserialization and result wrapping.
- **Zero Framework Lock-in**: No hard dependency on Axum—use it with any Tower-compatible server stack.

## Installation

Add `mcp-routing` to your `Cargo.toml`:

```toml
[dependencies]
mcp-routing = "0.1.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }
tower = { version = "0.5", features = ["util"] }
```

## Quick Start (with Axum)

```rust
use std::error::Error;
use axum::Router;
use mcp_routing::{
    McpRouter,
    types::mcp::{Implementation, tools::Tool},
};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize, Deserialize)]
struct EchoParams {
    value: String,
}

// Typed tool handler: arguments are automatically deserialized from JSON-RPC params
async fn echo(params: EchoParams) -> Result<String, String> {
    if params.value.is_empty() {
        return Err("Parameter 'value' cannot be empty".to_string());
    }
    Ok(params.value)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let server_info = Implementation::new("example-mcp-server", "0.1.0");

    let echo_tool = Tool {
        icons: Vec::new(),
        name: "echo".to_string(),
        title: Some("Echo Tool".to_string()),
        description: Some("Echoes the provided value back to the caller".to_string()),
        input_schema: json!({
            "type": "object",
            "properties": {
                "value": {
                    "type": "string",
                    "description": "The value to be echoed",
                }
            },
            "required": ["value"],
        }),
        output_schema: None,
        annotations: None,
        meta: None,
    };

    let mcp_router = McpRouter::new(server_info)
        .instructions("Example MCP server providing an echo tool")
        .register_tool(echo_tool, echo);

    // Nest the MCP router as a service in Axum
    let app = Router::new().nest_service("/mcp", mcp_router);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    println!("MCP Server listening on http://127.0.0.1:3000/mcp");
    axum::serve(listener, app).await?;
    Ok(())
}
```

## Stateless Protocol & Routing

`mcp-routing` targets the stateless [`2026-07-28` specification](https://modelcontextprotocol.io/docs/2026-07-28/) of the Model Context Protocol. Rather than establishing a persistent session via stateful initialization handshakes, each HTTP request is self-contained.

Incoming HTTP JSON-RPC requests are routed using HTTP headers:
- `Mcp-Method: server/discover` → Calls the server discovery handler.
- `Mcp-Method: tools/list` → Calls the tool discovery handler.
- `Mcp-Method: tools/call` and `Mcp-Name: <name>` → Invokes the registered tool `<name>`.

## Typed Tool Handlers

Tool handlers can accept:
- **No arguments**: `async fn my_tool() -> Result<String, String>`
- **Typed deserializable arguments**: `async fn my_tool(params: MyParams) -> Result<String, String>`

Return types can implement `IntoToolResult`:
- `String`, `&str`
- `ContentBlock`, `Vec<ContentBlock>`
- `CallToolResult`
- `Result<T, E>` where `T: IntoToolResult` and `E: Display`

## Running the Example

Run the included routing example:

```bash
cargo run --example routing
```

## Running Tests

```bash
cargo test
```

## Roadmap

See [ROADMAP.md](ROADMAP.md) for current MCP specification coverage, supported features, and planned capabilities.

## License

Licensed under the Apache License, Version 2.0 ([LICENSE](LICENSE) or <http://www.apache.org/licenses/LICENSE-2.0>).
