# Model Context Protocol (MCP) Roadmap (Archived)

> [!NOTE]
> **Status: Completed & Archived**
> All planned features across all 9 specification areas for the Model Context Protocol ([`2026-07-28` specification](https://modelcontextprotocol.io/docs/2026-07-28/)) have been fully implemented, verified with comprehensive unit and integration test suites, and documented.
>
> For current usage guides and feature documentation, refer to [`README.md`](../../README.md).

---

## Status Legend

| Symbol | Status | Description |
|:---:|:---|:---|
| ✅ | **Implemented** | Fully implemented and covered by unit/integration tests |
| 🟡 | **Partial** | Types or definitions exist, but runtime routing, validation, or handlers are missing |
| ❌ | **Not Implemented** | Feature is currently missing from the library |

---

## 1. HTTP Transport & Tower Routing

| Feature | Status | Details | Primary References |
|---|:---:|---|---|
| **Tower `Service` Implementation** | ✅ | [`McpRouter`](../../src/router/mod.rs) implements `tower::Service<Request<B>>` for any body `B: http_body::Body<Data = Bytes>` | [`src/router/mod.rs`](../../src/router/mod.rs) |
| **Streaming `ResponseBody`** | ✅ | Custom box-body [`ResponseBody`](../../src/body.rs) supporting `Bytes`, `Vec<u8>`, `String`, and conversions | [`src/body.rs`](../../src/body.rs) |
| **Header-Based Method Routing** | ✅ | Dispatches via `Mcp-Method` header (`server/discover`, `tools/list`, `tools/call`, `prompts/list`, `prompts/get`, `resources/list`, `resources/read`, `resources/templates/list`, `completion/complete`, `logging/setLevel`) | [`src/router/dispatch.rs`](../../src/router/dispatch.rs) |
| **Header-Based Tool Target Routing** | ✅ | Routes tool calls via `Mcp-Name: <name>` header | [`src/router/dispatch.rs`](../../src/router/dispatch.rs) |
| **Body-Based Method Dispatch Fallback** | ✅ | Fall back to `request.method` inside JSON-RPC body when `Mcp-Method` header is omitted | [`src/router/dispatch.rs`](../../src/router/dispatch.rs) |
| **Body-Based Tool Name Fallback** | ✅ | Fall back to `params.name` in [`CallToolParams`](../../src/types/mcp/tools/call.rs) when `Mcp-Name` header is omitted | [`src/router/dispatch.rs`](../../src/router/dispatch.rs) |
| **HTTP Verb & Media Type Negotiation** | ✅ | Validate `POST` method (return `405 Method Not Allowed` with `Allow: POST`) and `Content-Type: application/json` (return `415 Unsupported Media Type`) | [`src/router/service.rs`](../../src/router/service.rs) |
| **HTTP Caching Headers Propagation** | ✅ | Set HTTP `Cache-Control` (`public`/`private`, `max-age`) and `ETag` headers matching `ttl_ms` and `cache_scope` | [`src/body.rs`](../../src/body.rs), [`src/router/outcome.rs`](../../src/router/outcome.rs) |
| **Stateless Transport (SEP-2567)** | ✅ | Stateless HTTP transport without session affinity per MCP 2026-07-28 specification | [`src/router/service.rs`](../../src/router/service.rs), [`src/extract/mod.rs`](../../src/extract/mod.rs) |
| **Per-Request `_meta` Propagation** | ✅ | Extract and pass [`RequestMetaObject`](../../src/types/mcp/core/metadata.rs) (`clientInfo`, `protocolVersion`, `logLevel`, `progressToken`) via [`Meta`](../../src/extract/mod.rs) and [`RequestContext`](../../src/extract/context.rs) | [`src/extract/mod.rs`](../../src/extract/mod.rs), [`src/tools/mod.rs`](../../src/tools/mod.rs), [`src/prompts/mod.rs`](../../src/prompts/mod.rs) |
| **Context Extractors** | ✅ | Support [`Authorization`](../../src/extract/mod.rs), [`BearerAuth`](../../src/extract/mod.rs), [`RequestContext`](../../src/extract/context.rs), [`Meta`](../../src/extract/mod.rs), [`CurrentLoggingLevel`](../../src/extract/logging.rs), and Tower extractors ([`State<T>`](../../src/extract/mod.rs), [`Extension<T>`](../../src/extract/mod.rs), [`HeaderMap`](https://docs.rs/http/latest/http/header/struct.HeaderMap.html)) in handler signatures | [`src/extract/mod.rs`](../../src/extract/mod.rs), [`src/tools/mod.rs`](../../src/tools/mod.rs), [`src/prompts/mod.rs`](../../src/prompts/mod.rs) |

---

## 2. JSON-RPC 2.0 Protocol Compliance

| Feature | Status | Details | Primary References |
|---|:---:|---|---|
| **JSON-RPC Request Model** | ✅ | [`JsonRpcRequest<P>`](../../src/types/jsonrpc/request.rs) with flexible generic parameters | [`src/types/jsonrpc/request.rs`](../../src/types/jsonrpc/request.rs) |
| **Flexible Request ID Format** | ✅ | [`JsonRpcRequestId`](../../src/types/jsonrpc/mod.rs) supporting string and numeric IDs | [`src/types/jsonrpc/mod.rs`](../../src/types/jsonrpc/mod.rs) |
| **JSON-RPC Success Responses** | ✅ | [`JsonRpcResultResponse<R>`](../../src/types/jsonrpc/response.rs) serialized with standard `jsonrpc: "2.0"` | [`src/types/jsonrpc/response.rs`](../../src/types/jsonrpc/response.rs) |
| **JSON-RPC Error Structures** | ✅ | [`JsonRpcErrorResponse<E>`](../../src/types/jsonrpc/response.rs) with standard `error` objects and `id` mapping (including `null` on parse errors) | [`src/types/jsonrpc/response.rs`](../../src/types/jsonrpc/response.rs) |
| **Standard JSON-RPC Error Codes** | ✅ | Structured error mapping for `ParseError` (`-32700`), `InvalidRequest` (`-32600`), `MethodNotFound` (`-32601`), `InvalidParams` (`-32602`), and `InternalError` (`-32603`) | [`src/types/jsonrpc/error.rs`](../../src/types/jsonrpc/error.rs) |
| **JSON-RPC Batch Requests** | ✅ | Concurrent processing and batching array payloads `[JsonRpcRequest, ...]` over HTTP POST with JSON-RPC 2.0 compliance | [`src/router/service.rs`](../../src/router/service.rs), [`src/types/jsonrpc/batch.rs`](../../src/types/jsonrpc/batch.rs) |
| **JSON-RPC Notifications** | ✅ | Handling single and batch notifications without response generation, returning HTTP 204 No Content | [`src/types/jsonrpc/notification.rs`](../../src/types/jsonrpc/notification.rs), [`src/router/service.rs`](../../src/router/service.rs) |

---

## 3. Server Discovery & Metadata (`server/*`)

| Feature | Status | Details | Primary References |
|---|:---:|---|---|
| **`server/discover` Endpoint** | ✅ | Built-in endpoint via [`ServerConfig::dispatch_discover`](../../src/server/config.rs) | [`src/server/config.rs`](../../src/server/config.rs) |
| **Server Implementation Info** | ✅ | [`Implementation`](../../src/types/mcp/mod.rs) metadata (`name`, `version`, `title`, `description`, `website_url`, `icons`) | [`src/types/mcp/mod.rs`](../../src/types/mcp/mod.rs) |
| **Server Capabilities Advertisement** | ✅ | [`ServerCapabilities`](../../src/types/mcp/core/capabilities.rs) advertising tools, resources, prompts, completions, logging, and experimental | [`src/types/mcp/core/capabilities.rs`](../../src/types/mcp/core/capabilities.rs) |
| **Human-Readable Instructions** | ✅ | Router builder method [`.instructions()`](../../src/router/builder.rs) serialized into discovery response | [`src/router/builder.rs`](../../src/router/builder.rs) |
| **Supported Versions Configuration** | ✅ | Configurable supported protocol versions (defaults to `["2026-07-28"]`) | [`src/router/builder.rs`](../../src/router/builder.rs) |
| **Protocol Version Negotiation** | ✅ | Validate client `protocolVersion` in [`RequestMetaObject`](../../src/types/mcp/core/metadata.rs) against server `supported_versions` with configurable enforcement via [`.validate_protocol_version()`](../../src/router/builder.rs) | [`src/server/config.rs`](../../src/server/config.rs), [`src/router/builder.rs`](../../src/router/builder.rs) |
| **First-Class Discovery Handler** | ✅ | Unified handler via [`.discover()`](../../src/router/builder.rs) supporting typed extractors ([`RequestContext`](../../src/extract/context.rs), [`BearerAuth`](../../src/extract/mod.rs), [`Meta`](../../src/extract/mod.rs), [`Extension`](../../src/extract/mod.rs), [`State`](../../src/extract/mod.rs)) and flexible return conversions | [`src/server/provider.rs`](../../src/server/provider.rs), [`src/server/config.rs`](../../src/server/config.rs), [`src/router/builder.rs`](../../src/router/builder.rs) |

---

## 4. Tools Capability (`tools/*`)

| Feature | Status | Details | Primary References |
|---|:---:|---|---|
| **Tools Capability Flag** | ✅ | [`ToolsCapability`](../../src/types/mcp/core/capabilities.rs) advertising `list_changed` support | [`src/types/mcp/core/capabilities.rs`](../../src/types/mcp/core/capabilities.rs) |
| **Tool Definitions & Models** | ✅ | [`Tool`](../../src/types/mcp/tools/mod.rs) with schema, icons, titles, and [`ToolAnnotations`](../../src/types/mcp/tools/mod.rs) (`read_only_hint`, `destructive_hint`, `idempotent_hint`, `open_world_hint`) | [`src/types/mcp/tools/mod.rs`](../../src/types/mcp/tools/mod.rs) |
| **`tools/list` Endpoint** | ✅ | Built-in list handler in [`ToolRegistry`](../../src/tools/registry.rs) and custom handler support via [`.tools_list()`](../../src/router/builder.rs) with caching and pagination | [`src/tools/list.rs`](../../src/tools/list.rs), [`src/tools/registry.rs`](../../src/tools/registry.rs) |
| **`tools/call` Endpoint** | ✅ | Dispatches tool invocation to registered handlers in [`ToolRegistry`](../../src/tools/registry.rs) | [`src/tools/registry.rs`](../../src/tools/registry.rs) |
| **Typed Asynchronous Handlers** | ✅ | [`IntoToolHandler`](../../src/tools/mod.rs) for `async fn()` and `async fn(Args)` with Serde JSON deserialization | [`src/tools/mod.rs`](../../src/tools/mod.rs) |
| **Flexible Result Conversions** | ✅ | [`IntoToolResult`](../../src/tools/mod.rs) for `String`, `&str`, `ContentBlock`, `Vec<ContentBlock>`, `CallToolResult`, and `Result<T, E>` | [`src/tools/mod.rs`](../../src/tools/mod.rs) |
| **Tool Pagination (`cursor`)** | ✅ | Full pagination support via [`ListToolsParams`](../../src/types/mcp/tools/list.rs) and custom [`.tools_list()`](../../src/router/builder.rs) handlers returning [`ListToolsResult::with_next_cursor()`](../../src/types/mcp/tools/list.rs) | [`src/types/mcp/tools/list.rs`](../../src/types/mcp/tools/list.rs), [`src/tools/list.rs`](../../src/tools/list.rs) |
| **Input JSON Schema Pre-Validation** | ✅ | Pre-compiled JSON Schema validation of raw arguments against `tool.input_schema` prior to deserialization | [`src/tools/registry.rs`](../../src/tools/registry.rs) |
| **Structured Output Helpers** | ✅ | [`CallToolResult`](../../src/types/mcp/tools/call.rs) structured content constructors and fluent builders, [`Json<T>`](../../src/extract/json.rs) wrapper, generic [`CallToolResult<T>`](../../src/types/mcp/tools/call.rs) result conversions, and tuple returns | [`src/types/mcp/tools/call.rs`](../../src/types/mcp/tools/call.rs), [`src/extract/json.rs`](../../src/extract/json.rs), [`src/tools/mod.rs`](../../src/tools/mod.rs) |
| **First-Class Tools List Handler** | ✅ | Register custom async provider for listing tools generated or filtered per-request via [`.tools_list()`](../../src/router/builder.rs) | [`src/tools/list.rs`](../../src/tools/list.rs), [`src/tools/registry.rs`](../../src/tools/registry.rs), [`src/router/builder.rs`](../../src/router/builder.rs) |

---

## 5. Multi-Modal Content & Data Types

| Feature | Status | Details | Primary References |
|---|:---:|---|---|
| **Text Content** | ✅ | [`TextContent`](../../src/types/mcp/core/content.rs) with annotations and metadata | [`src/types/mcp/core/content.rs`](../../src/types/mcp/core/content.rs) |
| **Image Content** | ✅ | [`ImageContent`](../../src/types/mcp/core/content.rs) with base64 data and mimeType | [`src/types/mcp/core/content.rs`](../../src/types/mcp/core/content.rs) |
| **Audio Content** | ✅ | [`AudioContent`](../../src/types/mcp/core/content.rs) with base64 data and mimeType | [`src/types/mcp/core/content.rs`](../../src/types/mcp/core/content.rs) |
| **Embedded Resources** | ✅ | [`EmbeddedResource`](../../src/types/mcp/core/content.rs) (`TextResourceContents` and `BlobResourceContents`) | [`src/types/mcp/core/content.rs`](../../src/types/mcp/core/content.rs) |
| **Resource Links** | ✅ | [`ResourceLink`](../../src/types/mcp/core/content.rs) referencing external or hosted resources | [`src/types/mcp/core/content.rs`](../../src/types/mcp/core/content.rs) |
| **Content Annotations** | ✅ | [`ContentAnnotations`](../../src/types/mcp/core/content.rs) with `audience` ([`Role`](../../src/types/mcp/core/metadata.rs)) and `priority` (0.0–1.0) | [`src/types/mcp/core/content.rs`](../../src/types/mcp/core/content.rs) |
| **UI Icon Specifications** | ✅ | [`Icon`](../../src/types/mcp/core/metadata.rs) with `src`, `mime_type`, `sizes`, and [`IconTheme`](../../src/types/mcp/core/metadata.rs) | [`src/types/mcp/core/metadata.rs`](../../src/types/mcp/core/metadata.rs) |
| **Progress Tokens & Caching Scopes** | ✅ | [`ProgressToken`](../../src/types/mcp/core/metadata.rs) and [`CacheScope`](../../src/types/mcp/core/metadata.rs) | [`src/types/mcp/core/metadata.rs`](../../src/types/mcp/core/metadata.rs) |

---

## 6. Resources Capability (`resources/*`)

| Feature | Status | Details | Primary References |
|---|:---:|---|---|
| **Resource Capability Flag** | ✅ | [`ResourcesCapability`](../../src/types/mcp/core/capabilities.rs) advertised automatically upon registering resources or templates | [`src/types/mcp/core/capabilities.rs`](../../src/types/mcp/core/capabilities.rs), [`src/router/builder.rs`](../../src/router/builder.rs) |
| **Resource Definitions & Models** | ✅ | [`Resource`](../../src/types/mcp/resources/mod.rs), [`ResourceTemplate`](../../src/types/mcp/resources/mod.rs), [`ResourceAnnotations`](../../src/types/mcp/resources/mod.rs) data types and fluent builders | [`src/types/mcp/resources/mod.rs`](../../src/types/mcp/resources/mod.rs) |
| **`resources/list` Endpoint** | ✅ | Listing available direct resources with built-in handler and custom [`.resources_list()`](../../src/router/builder.rs) handler with pagination & caching | [`src/resources/list.rs`](../../src/resources/list.rs), [`src/resources/registry.rs`](../../src/resources/registry.rs) |
| **`resources/read` Endpoint** | ✅ | Reading text/blob resource content by direct URI or matching RFC 6570 URI templates | [`src/resources/read.rs`](../../src/resources/read.rs), [`src/resources/registry.rs`](../../src/resources/registry.rs) |
| **`resources/templates/list` Endpoint**| ✅ | Listing URI templates with built-in handler and custom [`.resource_templates_list()`](../../src/router/builder.rs) handler with caching | [`src/resources/templates.rs`](../../src/resources/templates.rs), [`src/resources/registry.rs`](../../src/resources/registry.rs) |
| **Typed Resource Handlers** | ✅ | [`IntoResourceHandler`](../../src/resources/mod.rs) and [`IntoResourceResult`](../../src/resources/mod.rs) for `async fn()` and `async fn(extractors..., uri: String)` | [`src/resources/mod.rs`](../../src/resources/mod.rs), [`src/router/builder.rs`](../../src/router/builder.rs) |

---

## 7. Prompts Capability (`prompts/*`)

| Feature | Status | Details | Primary References |
|---|:---:|---|---|
| **Prompt Capability Flag** | ✅ | [`PromptsCapability`](../../src/types/mcp/core/capabilities.rs) advertised automatically upon registering prompts | [`src/types/mcp/core/capabilities.rs`](../../src/types/mcp/core/capabilities.rs), [`src/router/builder.rs`](../../src/router/builder.rs) |
| **Prompt Definitions & Models** | ✅ | [`Prompt`](../../src/types/mcp/prompts/mod.rs), [`PromptArgument`](../../src/types/mcp/prompts/mod.rs), [`PromptMessage`](../../src/types/mcp/prompts/mod.rs) data types and builders | [`src/types/mcp/prompts/mod.rs`](../../src/types/mcp/prompts/mod.rs) |
| **`prompts/list` Endpoint** | ✅ | Built-in handler in [`PromptRegistry`](../../src/prompts/registry.rs) and custom handler support via [`.prompts_list()`](../../src/router/builder.rs) with caching | [`src/prompts/list.rs`](../../src/prompts/list.rs), [`src/prompts/registry.rs`](../../src/prompts/registry.rs) |
| **`prompts/get` Endpoint** | ✅ | Retrieving prompt messages with argument substitution, multi-modal content, and caching | [`src/prompts/mod.rs`](../../src/prompts/mod.rs), [`src/router/dispatch.rs`](../../src/router/dispatch.rs) |
| **Typed Prompt Handlers** | ✅ | [`IntoPromptHandler`](../../src/prompts/mod.rs) and [`IntoPromptResult`](../../src/prompts/mod.rs) for `async fn()` and `async fn(Args)` | [`src/prompts/mod.rs`](../../src/prompts/mod.rs) |
| **First-Class Prompts List Handler** | ✅ | Register custom async provider for listing prompts generated or filtered per-request via [`.prompts_list()`](../../src/router/builder.rs) | [`src/prompts/list.rs`](../../src/prompts/list.rs), [`src/prompts/registry.rs`](../../src/prompts/registry.rs), [`src/router/builder.rs`](../../src/router/builder.rs) |

---

## 8. Completions Capability (`completion/*`)

| Feature | Status | Details | Primary References |
|---|:---:|---|---|
| **Completions Capability Flag** | ✅ | [`CompletionsCapability`](../../src/types/mcp/core/capabilities.rs) advertised automatically upon registering completion handlers | [`src/types/mcp/core/capabilities.rs`](../../src/types/mcp/core/capabilities.rs), [`src/router/builder.rs`](../../src/router/builder.rs) |
| **`completion/complete` Endpoint** | ✅ | Autocompletion endpoint for prompt arguments and resource reference values with caching and pagination | [`src/completion/mod.rs`](../../src/completion/mod.rs), [`src/completion/registry.rs`](../../src/completion/registry.rs) |
| **Completion Types & Handlers** | ✅ | [`CompleteRequest`](../../src/types/mcp/completion/mod.rs), [`CompleteResult`](../../src/types/mcp/completion/mod.rs), [`IntoCompletionHandler`](../../src/completion/mod.rs), and registration APIs ([`.completion()`](../../src/router/builder.rs), [`.register_prompt_completion()`](../../src/router/builder.rs), [`.register_resource_completion()`](../../src/router/builder.rs)) | [`src/types/mcp/completion/mod.rs`](../../src/types/mcp/completion/mod.rs), [`src/completion/mod.rs`](../../src/completion/mod.rs), [`src/router/builder.rs`](../../src/router/builder.rs) |

---

## 9. Logging & Diagnostics (`logging/*`)

| Feature | Status | Details | Primary References |
|---|:---:|---|---|
| **Logging Severity Levels** | ✅ | [`LoggingLevel`](../../src/types/mcp/core/metadata.rs) enum matching RFC-5424 severities with parsing, display, comparison, and `tracing::Level` mapping | [`src/types/mcp/core/metadata.rs`](../../src/types/mcp/core/metadata.rs) |
| **`logging/setLevel` Endpoint** | ✅ | Typed endpoint & notifications allowing clients to dynamically request server logging threshold, with custom async handlers and extractors | [`src/logging/mod.rs`](../../src/logging/mod.rs), [`src/logging/registry.rs`](../../src/logging/registry.rs) |
| **Per-Request `logLevel` Handling**| ✅ | Per-request `_meta.io.modelcontextprotocol/logLevel` extraction into `LoggingLevel` / `Option<LoggingLevel>`, server dynamic `CurrentLoggingLevel`, and `RequestContext::should_log` | [`src/extract/logging.rs`](../../src/extract/logging.rs), [`src/extract/context.rs`](../../src/extract/context.rs) |

---

## Phased Development Plan

```mermaid
gantt
    title Phased Implementation Plan
    dateFormat  YYYY-MM-DD
    section Phase 1: Robustness & Spec Errors
    Body-based method fallback         :done, p1_1, 2026-08-15, 3d
    Tool name param fallback           :done, p1_2, after p1_1, 2d
    JSON-RPC error code responses      :done, p1_3, after p1_2, 3d
    HTTP verb & Content-Type validation:done, p1_4, after p1_3, 2d
    Cache-Control header propagation   :done, p1_5, after p1_4, 2d
    section Phase 2: Core Capabilities
    Prompts (prompts/list, prompts/get):done, p2_1, after p1_5, 5d
    Resources (resources/list, read)   :done, p2_2, after p2_1, 6d
    Completions (completion/complete)  :done, p2_3, after p2_2, 4d
    section Phase 3: Ergonomics & Extensibility
    Session & Context Extractors       :done, p3_1, 2026-08-15, 3d
    JSON Schema Input Pre-Validation   :done, p3_2, after p2_2, 4d
    Dynamic Tool/Prompt Providers      :done, p3_3, after p3_2, 4d
    Logging & Diagnostics (setLevel)   :done, p3_4, after p3_3, 3d
```
