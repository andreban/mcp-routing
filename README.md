# mcp-routing

A [Tower](https://crates.io/crates/tower)-native routing library for building [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) servers in Rust.

> **Note:** `mcp-routing` exclusively supports the **stateless** version of the Model Context Protocol ([`2026-07-28` specification](https://modelcontextprotocol.io/docs/2026-07-28/)). It uses request-based discovery (`server/discover`) and direct tool execution, and does **not** support previous stateful protocol versions (e.g. 2024-11-05 `initialize` lifecycle).

`mcp-routing` provides a composable, framework-agnostic [`McpRouter`] that implements [`tower::Service`]. It can be plugged directly into [Axum](https://crates.io/crates/axum), [Hyper](https://crates.io/crates/hyper), or any custom Tower middleware pipeline.

## Features

- **Stateless MCP (`2026-07-28`)**: Built specifically for the 2026-07-28 MCP specification featuring `server/discover`, `tools/*`, `prompts/*`, `resources/*`, `completion/*`, and `logging/*`.
- **Tower-Native**: Implements `tower::Service` for any HTTP request body implementing `http_body::Body<Data = Bytes>`.
- **Header & Body Routing**: Dispatches requests via standard `Mcp-Method`, `Mcp-Name`, and `Mcp-Uri` headers with automatic fallback to JSON-RPC body parameters.
- **Typed Asynchronous Handlers**: Register async Rust functions with automatic JSON-RPC argument deserialization, structured output, and error mapping.
- **Rich Extractors**: Extract `BearerAuth`, `State<T>`, `Extension<T>`, `Meta`, `CurrentLoggingLevel`, `RequestContext`, and registered registries.
- **Dynamic Providers**: Dynamically generate or filter discovery metadata, tools, prompts, resources, and templates per request.
- **Input Pre-Validation**: Pre-compiled JSON Schema validation for tool arguments prior to deserialization.
- **HTTP Caching Directives**: Automatic generation of `Cache-Control` (`public`/`private`, `max-age`) and `ETag` headers based on metadata `ttl_ms` and `cache_scope`.
- **JSON-RPC 2.0 Batches & Notifications**: Full support for concurrent batch request processing and notifications returning HTTP 204 No Content.
- **Zero Framework Lock-in**: Usable with Axum, Hyper, or any Tower-compatible server stack.

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
    let server_info = Implementation::new("example-mcp-server", "1.0.0");

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
        // Cache server/discover response for 1 hour publicly:
        // generates HTTP `Cache-Control: public, max-age=3600` and `ETag`
        .server_discover_cache(Some(3_600_000), Some(mcp_routing::types::mcp::CacheScope::Public))
        // Cache tools/list response for 5 minutes publicly:
        // generates HTTP `Cache-Control: public, max-age=300` and `ETag`
        .tools_list_cache(Some(300_000), Some(mcp_routing::types::mcp::CacheScope::Public))
        .register_tool(echo_tool, echo);

    // Nest the MCP router as a service in Axum
    let app = Router::new().nest_service("/mcp", mcp_router);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    println!("MCP Server listening on http://127.0.0.1:3000/mcp");
    axum::serve(listener, app).await?;
    Ok(())
}
```

## Protocol & Routing

`mcp-routing` targets the stateless [`2026-07-28` specification](https://modelcontextprotocol.io/docs/2026-07-28/) of the Model Context Protocol. Each HTTP request is self-contained.

Incoming HTTP JSON-RPC requests are dispatched using headers with body fallback:

| MCP Method | HTTP Headers | Body Fallback | Handler / Purpose |
|---|---|---|---|
| `server/discover` | `Mcp-Method: server/discover` | `method: "server/discover"` | Server discovery & capability negotiation |
| `tools/list` | `Mcp-Method: tools/list` | `method: "tools/list"` | Discovers registered tools & schemas |
| `tools/call` | `Mcp-Method: tools/call`<br>`Mcp-Name: <name>` | `method: "tools/call"`<br>`params.name: "<name>"` | Executes registered tool `<name>` |
| `prompts/list` | `Mcp-Method: prompts/list` | `method: "prompts/list"` | Discovers registered prompt templates |
| `prompts/get` | `Mcp-Method: prompts/get`<br>`Mcp-Name: <name>` | `method: "prompts/get"`<br>`params.name: "<name>"` | Retrieves prompt messages & fills arguments |
| `resources/list` | `Mcp-Method: resources/list` | `method: "resources/list"` | Discovers direct resources |
| `resources/read` | `Mcp-Method: resources/read`<br>`Mcp-Uri: <uri>` | `method: "resources/read"`<br>`params.uri: "<uri>"` | Reads resource content or matches URI template |
| `resources/templates/list` | `Mcp-Method: resources/templates/list` | `method: "resources/templates/list"` | Discovers RFC 6570 resource templates |
| `completion/complete` | `Mcp-Method: completion/complete` | `method: "completion/complete"` | Autocompletes prompt arguments & URI templates |

---

## Capabilities & Handlers

### 1. Tools (`tools/*`)

Register tools with static definitions or dynamic list providers:

```rust
// Typed tool handler with extractors and structured output
async fn query_db(
    auth: BearerAuth,
    params: QueryParams,
) -> Result<Json<DbResult>, String> {
    // ...
    Ok(Json(DbResult { rows: vec![] }))
}

let router = McpRouter::new(server_info)
    .validate_tool_inputs(true) // Pre-validates arguments against input_schema
    .register_tool(db_tool, query_db);
```

Supported return types ([`IntoToolResult`](src/tools/mod.rs)):
- `String`, `&str`, `ContentBlock`, `Vec<ContentBlock>`
- `CallToolResult<T>` (fluent builder for structured data, text, and multimodal content)
- `Json<T>` and `serde_json::Value` (automatic structured output)
- `(Json<T>, &str)`, `(Json<T>, String)`, `(Json<T>, Vec<ContentBlock>)` (structured data + content blocks)
- `Result<T, E>` where `T: IntoToolResult` and `E: Display`

### 2. Prompts (`prompts/*`)

Register parameterized prompt templates:

```rust
async fn code_review(params: ReviewParams) -> Result<Vec<PromptMessage>, String> {
    Ok(vec![
        PromptMessage::user_text(format!("Review this code:\n\n{}", params.code)),
        PromptMessage::assistant_text("I will analyze the code for quality, performance, and security."),
    ])
}

let router = McpRouter::new(server_info)
    .register_prompt(review_prompt, code_review);
```

### 3. Resources (`resources/*`)

Register direct resources or RFC 6570 URI templates:

```rust
// Direct text resource
let router = McpRouter::new(server_info)
    .register_resource_text(
        "config://app",
        "App Config",
        Some("Application configuration JSON"),
        r#"{"debug": false}"#,
        "application/json",
    );

// Dynamic RFC 6570 URI template handler
let user_template = ResourceTemplate::new("users://{user_id}/profile", "User Profile")
    .with_description("Returns user profile data");

let router = router.register_resource_template(user_template, |uri: String| async move {
    ReadResourceResult::text(uri, "User Profile Content", Some("application/json"))
});
```

### 4. Completions (`completion/*`)

Provide autocompletion for prompt arguments and resource template variables:

```rust
let router = McpRouter::new(server_info)
    .register_prompt_completion("code_review", "language", |_ctx, query| async move {
        let languages = vec!["rust", "python", "typescript", "go"];
        languages
            .into_iter()
            .filter(|l| l.starts_with(&query))
            .collect::<Vec<_>>()
    });
```

### 5. Logging & Diagnostics (`logging/*`)

Configure server logging capabilities and initial default thresholds:

```rust
let router = McpRouter::new(server_info)
    .logging_level(LoggingLevel::Info);
```

Inspect per-request `_meta.io.modelcontextprotocol/logLevel` and current server log thresholds in any tool or handler:

```rust
async fn process_task(
    opt_level: Option<LoggingLevel>,
    current_level: CurrentLoggingLevel,
    params: TaskParams,
) -> Result<String, String> {
    let effective = opt_level.unwrap_or(current_level.level());
    Ok(format!("Executing with log level: {effective}"))
}
```

---

## Request Extractors

Handlers can accept up to 5 Tower and MCP extractors in their signatures:

| Extractor | Source / Description |
|---|---|
| [`BearerAuth`](src/extract/mod.rs) | Bearer token from `Authorization: Bearer <token>` header |
| [`Authorization`](src/extract/mod.rs) | Raw `Authorization` header |
| [`State<T>`](src/extract/mod.rs) | Application state shared across Tower layers / Axum handlers (`.with_state(state)`) |
| [`Extension<T>`](src/extract/mod.rs) | Type-safe request extensions from Tower middleware |
| [`Meta`](src/extract/mod.rs) / [`RequestMetaObject`](src/types/mcp/core/metadata.rs) | Client info, protocol version, log level, progress tokens |
| [`CurrentLoggingLevel`](src/extract/logging.rs) | Dynamic server logging threshold |
| [`LoggingLevel`](src/types/mcp/core/metadata.rs) / `Option<LoggingLevel>` | Per-request log level from `_meta.io.modelcontextprotocol/logLevel` |
| [`RequestContext`](src/extract/context.rs) | Full MCP request context (headers, extensions, metadata) |
| [`RegisteredTools`](src/extract/mod.rs) | Injected registry of registered tools (useful in custom `.tools_list()`) |
| [`RegisteredPrompts`](src/extract/mod.rs) | Injected registry of registered prompts (useful in custom `.prompts_list()`) |
| [`RegisteredResources`](src/extract/mod.rs) | Injected registry of direct resources (useful in custom `.resources_list()`) |
| [`RegisteredResourceTemplates`](src/extract/mod.rs) | Injected registry of resource templates |
| [`HeaderMap`](https://docs.rs/http/latest/http/header/struct.HeaderMap.html) | Raw HTTP request headers |

---

## Examples

Run any of the included examples with `cargo run --example <name>`:

| Example | Command | Description |
|---|---|---|
| **Basic** | `cargo run --example basic` | Minimal starter MCP server embedded in Axum |
| **Resources** | `cargo run --example resources` | Direct text/blob resources, RFC 6570 URI templates, and caching |
| **Structured Output** | `cargo run --example structured_output` | Structured outputs via `Json<T>`, `output_schema`, annotations, and error wrappers |
| **Caching** | `cargo run --example caching` | Public and private caching directives (`Cache-Control`, `ETag`) |
| **Prompts** | `cargo run --example prompts` | Parameterized and multi-turn prompt templates with role messages |
| **Completions** | `cargo run --example completions` | Autocompletion for prompt arguments and resource template variables |
| **Extractors** | `cargo run --example extractors` | Sharing application state (`State<T>`), session IDs, and auth tokens |
| **Discovery** | `cargo run --example discovery` | Dynamic capability advertisement and contextual server instructions |
| **Logging** | `cargo run --example logging` | Server logging level advertisement and per-request log level handling |

---

## Running Tests

Run the complete test suite:

```bash
cargo test
```

## Specification & Roadmap

All planned capabilities across all 9 specification sections of the Model Context Protocol ([`2026-07-28`](https://modelcontextprotocol.io/docs/2026-07-28/)) are fully implemented. For historical development tracking and the phased implementation breakdown, see [docs/archive/ROADMAP.md](docs/archive/ROADMAP.md).

## License

Licensed under the Apache License, Version 2.0 ([LICENSE](LICENSE) or <http://www.apache.org/licenses/LICENSE-2.0>).
