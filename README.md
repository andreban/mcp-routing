# stateless-mcp

A [Tower](https://crates.io/crates/tower)-native routing library for building [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) servers in Rust.

> **Note:** `stateless-mcp` exclusively supports the **stateless** version of the Model Context Protocol ([`2026-07-28` specification](https://modelcontextprotocol.io/specification/2026-07-28)). It uses request-based discovery (`server/discover`) and direct tool execution, and does **not** support previous stateful protocol versions (e.g. 2024-11-05 `initialize` lifecycle).

`stateless-mcp` provides a composable, framework-agnostic [`McpRouter`] that implements [`tower::Service`]. It can be plugged directly into [Axum](https://crates.io/crates/axum), [Hyper](https://crates.io/crates/hyper), or any custom Tower middleware pipeline.

## Features

- **Stateless MCP (`2026-07-28`)**: Built specifically for the 2026-07-28 MCP specification featuring `server/discover`, `tools/*`, `prompts/*`, `resources/*`, `completion/*`, and `subscriptions/listen`. Features deprecated in this revision (Logging, Roots, Sampling) are not supported.
- **Tower-Native**: Implements `tower::Service` for any HTTP request body implementing `http_body::Body<Data = Bytes>`.
- **Header & Body Validation**: Requires the standard `Mcp-Method` and `Mcp-Name` headers and verifies they exactly match the JSON-RPC body (`-32020 HeaderMismatch` otherwise).
- **Typed Asynchronous Handlers**: Register async Rust functions with automatic JSON-RPC argument deserialization, structured output, and error mapping.
- **Rich Extractors**: Extract `BearerAuth`, `State<T>`, `Extension<T>`, `Meta`, `RequestContext`, `RequestState` / `InputResponses` (multi round-trip), `Subscription`, and registered registries.
- **Dynamic Providers**: Dynamically generate or filter discovery metadata, tools, prompts, resources, and templates per request.
- **Input Pre-Validation**: Pre-compiled JSON Schema validation for tool arguments prior to deserialization; failures are returned as `isError: true` tool results.
- **Multi Round-Trip Requests**: Handlers can return `InputRequiredResult` to request elicitation from the client and resume on retry with `RequestState` / `InputResponses`.
- **Subscriptions**: `subscriptions/listen` streams over SSE, acknowledging only notification types backed by declared capabilities.
- **HTTP Caching Directives**: Automatic generation of `Cache-Control` (`public`/`private`, `max-age`) and `ETag` headers from each result's `ttlMs` and `cacheScope`; `input_required` and retry results are sent with `no-store`.
- **Single-Message Framing & Notifications**: One JSON-RPC message per POST as required by Streamable HTTP (batch arrays are rejected with `-32600`); notifications return HTTP 202 Accepted.
- **Zero Framework Lock-in**: Usable with Axum, Hyper, or any Tower-compatible server stack.

## Installation

Add `stateless-mcp` to your `Cargo.toml`:

```toml
[dependencies]
stateless-mcp = "0.1.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }
tower = { version = "0.5", features = ["util"] }
```

## Quick Start (with Axum)

```rust
use std::error::Error;
use axum::Router;
use stateless_mcp::{
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
        .server_discover_cache(Some(3_600_000), Some(stateless_mcp::types::mcp::CacheScope::Public))
        // Cache tools/list response for 5 minutes publicly:
        // generates HTTP `Cache-Control: public, max-age=300` and `ETag`
        .tools_list_cache(Some(300_000), Some(stateless_mcp::types::mcp::CacheScope::Public))
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

`stateless-mcp` targets the stateless [`2026-07-28` specification](https://modelcontextprotocol.io/specification/2026-07-28) of the Model Context Protocol. Each HTTP request is self-contained.

Every request carries its method (and, where applicable, its target) both in the JSON-RPC body and in mirrored HTTP headers; the values must match exactly:

| MCP Method | HTTP Headers | JSON-RPC Body | Handler / Purpose |
|---|---|---|---|
| `server/discover` | `Mcp-Method: server/discover` | `method: "server/discover"` | Server discovery & capability negotiation |
| `tools/list` | `Mcp-Method: tools/list` | `method: "tools/list"` | Discovers registered tools & schemas |
| `tools/call` | `Mcp-Method: tools/call`<br>`Mcp-Name: <name>` | `method: "tools/call"`<br>`params.name: "<name>"` | Executes registered tool `<name>` |
| `prompts/list` | `Mcp-Method: prompts/list` | `method: "prompts/list"` | Discovers registered prompt templates |
| `prompts/get` | `Mcp-Method: prompts/get`<br>`Mcp-Name: <name>` | `method: "prompts/get"`<br>`params.name: "<name>"` | Retrieves prompt messages & fills arguments |
| `resources/list` | `Mcp-Method: resources/list` | `method: "resources/list"` | Discovers direct resources |
| `resources/read` | `Mcp-Method: resources/read`<br>`Mcp-Name: <uri>` | `method: "resources/read"`<br>`params.uri: "<uri>"` | Reads resource content or matches URI template |
| `resources/templates/list` | `Mcp-Method: resources/templates/list` | `method: "resources/templates/list"` | Discovers RFC 6570 resource templates |
| `completion/complete` | `Mcp-Method: completion/complete` | `method: "completion/complete"` | Autocompletes prompt arguments & URI templates |
| `subscriptions/listen` | `Mcp-Method: subscriptions/listen` | `method: "subscriptions/listen"` | Opens an SSE stream of change notifications |

### Protocol Enforcement

Requests that do not follow the specification are rejected before any handler runs:

| Condition | Response |
|---|---|
| Missing or mismatched `MCP-Protocol-Version`, `Mcp-Method`, `Mcp-Name`, or `Mcp-Param-*` header (including malformed `=?base64?...?=` values) | `400`, `-32020` (`HeaderMismatch`) |
| Unsupported protocol version | `400`, `-32022` with `data.supported` / `data.requested` |
| Missing `_meta["io.modelcontextprotocol/protocolVersion"]` or `_meta["io.modelcontextprotocol/clientCapabilities"]` | `400`, `-32602` |
| Batch array, missing `jsonrpc: "2.0"` or `method`, `null` or fractional `id`, or a request method sent without an `id` | `400`, `-32600` |
| Unknown method | `404`, `-32601` |
| `Origin` header not allowed (only loopback origins are allowed unless `.allowed_origins(...)` is configured) | `403` |
| `input_required` result asking for elicitation the client did not declare | `400`, `-32021` |

Every result carries `_meta["io.modelcontextprotocol/serverInfo"]`.

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

// Arguments are validated against `db_tool.input_schema` before `query_db` runs;
// validation failures are returned as `isError: true` tool results.
let router = McpRouter::new(server_info).register_tool(db_tool, query_db);
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
// Direct resource
let router = McpRouter::new(server_info).register_resource(("config://app", "App Config"), || async {
    ReadResourceResult::text("config://app", r#"{"debug": false}"#, Some("application/json"))
});

// Dynamic RFC 6570 URI template handler
let user_template = ResourceTemplate::new("users://{user_id}/profile", "User Profile");

let router = router.register_resource_template(user_template, |uri: String| async move {
    ReadResourceResult::text(uri, "User Profile Content", Some("application/json"))
});
```

### 4. Completions (`completion/*`)

Provide autocompletion for prompt arguments and resource template variables:

```rust
let router = McpRouter::new(server_info).register_prompt_arg_completion(
    "code_review",
    "language",
    |arg: CompleteArgument| async move {
        ["rust", "python", "typescript", "go"]
            .into_iter()
            .filter(|l| l.starts_with(arg.value.as_str()))
            .collect::<Vec<_>>()
    },
);
```

### 5. Multi Round-Trip Requests (elicitation)

`tools/call`, `prompts/get`, and `resources/read` handlers can ask the client for input by returning an `InputRequiredResult`; the client retries with `requestState` and `inputResponses`. The client must declare the `elicitation` capability (and the requested mode), otherwise the request fails with `-32021`.

```rust
async fn delete_all(state: Option<RequestState>, responses: Option<InputResponses>) -> CallToolResult {
    let confirmed = responses
        .and_then(|r| r.get_result::<serde_json::Value>("confirm").ok().flatten())
        .is_some_and(|answer| answer["action"] == "accept");
    if state.is_some() && confirmed {
        return CallToolResult::text("Deleted everything");
    }
    InputRequiredResult::new()
        .with_request_state("delete_all")
        .with_input_request(
            "confirm",
            InputRequest::elicitation(&json!({
                "mode": "form",
                "message": "Delete everything?",
                "requestedSchema": { "type": "object", "properties": {} }
            }))
            .unwrap(),
        )
        .into_tool_result()
}
```

`requestState` round-trips through the client, so treat it as untrusted input and integrity-protect it if it influences authorization or business logic.

### 6. Subscriptions (`subscriptions/listen`)

Declare the notification types the server emits, then stream them from a listen handler. The `Subscription` extractor provides the acknowledged filter and the subscription ID that every notification must carry:

```rust
let router = McpRouter::new(server_info)
    .capabilities(ServerCapabilities::empty().with_tools(Some(true)))
    .subscriptions_listen(|subscription: Subscription| async move {
        // Stream only `subscription.notifications` types, tagging each with
        // `subscription.meta()`; see examples/subscriptions for a full stream.
        ResponseBody::empty()
    });
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
| [`Meta`](src/extract/mod.rs) / [`RequestMetaObject`](src/types/mcp/core/metadata.rs) | Client info, protocol version, progress tokens |
| [`RequestContext`](src/extract/context.rs) | Full MCP request context (headers, extensions, metadata) |
| [`RequestState`](src/extract/mrtr.rs) / [`InputResponses`](src/extract/mrtr.rs) | `requestState` and `inputResponses` of a multi round-trip retry (use `Option<_>` on the first attempt) |
| [`Subscription`](src/extract/subscription.rs) | Subscription ID and acknowledged notification filter (`subscriptions/listen` handlers only) |
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
| **Extractors** | `cargo run --example extractors` | Sharing application state (`State<T>`) between Axum routes and MCP handlers |
| **Discovery** | `cargo run --example discovery` | Dynamic capability advertisement and contextual server instructions |
| **Subscriptions** | `cargo run --example subscriptions` | Event-driven `subscriptions/listen` notification streams |
| **Movie Watchlist** | `cargo run --example movie_watchlist` | End-to-end server with auth, dynamic discovery, prompts, resources, and completions |

---

## Running Tests

Run the complete test suite:

```bash
cargo test
```

## Specification Compliance

`stateless-mcp` targets the Model Context Protocol [`2026-07-28`](https://modelcontextprotocol.io/specification/2026-07-28) specification for servers using the Streamable HTTP transport. Known compliance gaps are tracked as [GitHub issues](https://github.com/andreban/stateless-mcp/issues). The documents in [docs/archive/](docs/archive/) are historical and no longer reflect the current implementation.

## License

Licensed under the Apache License, Version 2.0 ([LICENSE](LICENSE) or <http://www.apache.org/licenses/LICENSE-2.0>).
