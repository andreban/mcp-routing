# MCP Specification Compliance Review (2026-07-28)

**Specification Version**: `2026-07-28`  
**Specification References**: [Model Context Protocol Specification (2026-07-28)](https://modelcontextprotocol.io/specification/2026-07-28/) & [Schema (`schema.ts`)](https://raw.githubusercontent.com/modelcontextprotocol/specification/main/schema/2026-07-28/schema.ts)  
**Codebase**: `mcp-routing` (Rust)  
**Date**: August 2026  

---

## Executive Summary

A comprehensive compliance audit of `mcp-routing` against the Model Context Protocol (MCP) `2026-07-28` specification was performed. While `mcp-routing` provides a solid routing framework with caching and typed handlers, there are several key areas where the current implementation diverges from normative specification requirements (`MUST` / `SHOULD`), particularly around **Streamable HTTP transport headers**, **HTTP status code semantics**, **JSON-RPC error code mappings**, **serialization tags**, and **2026-07-28 protocol changes** (such as SEP-2567, SEP-2575, and SEP-2577).

This document itemizes all required changes and recommendations, categorized by subsystem and prioritized by conformance impact.

---

## Priority 1: High (Normative Spec Compliance Violations)

### 1.1 Streamable HTTP: HTTP Status Code Corrections

- **Spec Requirement**:
  - **Notifications**: "A server that successfully accepts a notification MUST respond with HTTP `202 Accepted` and an empty body." ([`streamable-http.md`](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/streamable-http.html#post-requests))
  - **Unknown Method**: "If the JSON-RPC request specifies a method that the server does not recognize, the server MUST return HTTP `404 Not Found` with a JSON-RPC error response with code `-32601` (Method not found)." ([`streamable-http.md`](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/streamable-http.html#json-rpc-error-handling-and-http-status-codes))
  - **Protocol & Header Errors**: Errors `-32020` (`HeaderMismatch`), `-32021` (`MissingRequiredClientCapability`), `-32022` (`UnsupportedProtocolVersion`), and malformed `_meta` parameters MUST return HTTP `400 Bad Request`. ([`streamable-http.md`](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/streamable-http.html#json-rpc-error-handling-and-http-status-codes))
- **Current Implementation**:
  - `src/router/service.rs:113, 130` returns `204 No Content` (`StatusCode::NO_CONTENT`) for notifications.
  - `src/router/service.rs` wraps all JSON-RPC error responses (including `-32601`, `-32020`, `-32022`) in HTTP `200 OK`.
- **Action Items**:
  - [x] Update notification responses to return HTTP `202 Accepted` (`StatusCode::ACCEPTED`) with an empty body.
  - [x] In `src/router/service.rs` and `src/body.rs`, inspect JSON-RPC error codes and map them to HTTP status codes:
    - `-32700` (Parse error) -> `400 Bad Request`
    - `-32600` (Invalid Request) -> `400 Bad Request`
    - `-32601` (Method not found) -> `404 Not Found`
    - `-32020` (Header mismatch) -> `400 Bad Request`
    - `-32021` (Missing required client capability) -> `400 Bad Request`
    - `-32022` (Unsupported protocol version) -> `400 Bad Request`
    - Application & standard JSON-RPC results and errors -> `200 OK`
  - [x] Replace `-32601` (`MethodNotFound`) with `-32602` (`InvalidParams`) for missing target items (tools in `tools/call`, prompts in `prompts/get`, resources in `resources/read`, and completion targets in `completion/complete`).

---

### 1.2 Streamable HTTP: `MCP-Protocol-Version` Header Enforcement

- **Spec Requirement**:
  - "Every POST request to the MCP endpoint MUST include an `MCP-Protocol-Version` header." ([`streamable-http.md`](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/streamable-http.html#headers))
  - "If the header is omitted, or if its value does not match `_meta[\"io.modelcontextprotocol/protocolVersion\"]` when both are present, the server MUST return HTTP `400 Bad Request` with error code `-32020` (`HeaderMismatch`)."
  - "If the protocol version is not supported, the server MUST return HTTP `400 Bad Request` with error code `-32022` (`UnsupportedProtocolVersion`) and payload `data: { supported: string[], requested: string }`."
- **Current Implementation**:
  - Protocol version validation is only evaluated conditionally inside `server/discover` via `src/server/discover.rs:validate_protocol_version`, and it returns `-32602` (`Invalid params`) inside HTTP `200 OK`.
  - The HTTP header `MCP-Protocol-Version` is neither required nor validated in `src/router/service.rs`.
- **Action Items**:
  - [x] Validate `MCP-Protocol-Version` HTTP header on all incoming POST requests in `src/router/service.rs`.
  - [x] Verify consistency between HTTP `MCP-Protocol-Version` header and `params._meta["io.modelcontextprotocol/protocolVersion"]` (returning `-32020` on mismatch).
  - [x] Implement `-32022` (`UnsupportedProtocolVersionError`) returning the list of supported server versions in `data.supported` and the client's version in `data.requested`.

---

### 1.3 Streamable HTTP: Strict Header Routing & Verification (`Mcp-Method`, `Mcp-Name`)

- **Spec Requirement**:
  - `Mcp-Method` header is **REQUIRED** on all POST requests and MUST match `request.method`. ([`streamable-http.md`](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/streamable-http.html#headers))
  - `Mcp-Name` header is **REQUIRED** for `tools/call`, `resources/read`, and `prompts/get`, and MUST match `request.params.name` or `request.params.uri`.
  - If headers are missing or mismatched with body values, server MUST reject the request with HTTP `400 Bad Request` and JSON-RPC error `-32020` (`HeaderMismatch`).
- **Current Implementation**:
  - `src/utils.rs:resolve_tool_name`, `resolve_prompt_name`, `resolve_resource_uri` implement fallback resolution (`header.or(body)`), meaning requests without headers or with conflicting headers silently pass.
- **Action Items**:
  - [x] Add strict validation: if `Mcp-Method` or `Mcp-Name` / `Mcp-Uri` is missing or conflicts with body values, return HTTP `400 Bad Request` with `-32020` `HeaderMismatch`.
  - [x] Strictly enforce header routing requirements directly per 2026-07-28 Streamable HTTP specification.

---

### 1.4 Streamable HTTP: Base64 Sentinel Value Decoding

- **Spec Requirement**:
  - "Values of `Mcp-Name` and `Mcp-Param-*` headers containing non-ASCII characters, spaces, or control characters MUST use RFC 2047-style sentinel encoding: `=?base64?<base64-encoded-utf8-bytes>?=`." ([`streamable-http.md`](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/streamable-http.html#header-encoding))
  - "Servers MUST decode sentinel values before matching against tool names, resource URIs, or parameter values."
- **Current Implementation**:
  - `src/utils.rs:extract_header_name`, `extract_header_uri` read the header value directly as plain UTF-8 strings without decoding `=?base64?...?=`.
- **Action Items**:
  - [x] Implement RFC 2047-style sentinel decoding helper in `src/utils.rs`: `decode_sentinel_header(raw_value: &str) -> Cow<'_, str>`.
  - [x] Apply sentinel decoding when extracting `Mcp-Name` and `Mcp-Uri` headers.

---

### 1.5 JSON-RPC Error Codes: Target Not Found (`-32602` vs `-32601`)

- **Spec Requirement**:
  - "Codes defined by earlier protocol versions remain reserved and are never reused: `-32002` (resource not found, 2025-11-25 and earlier; replaced by `-32602`)." ([`schema.ts:430-432`](https://raw.githubusercontent.com/modelcontextprotocol/specification/main/schema/2026-07-28/schema.ts))
  - Unknown tools in `tools/call`, unknown prompts in `prompts/get`, and unknown resources in `resources/read` MUST return JSON-RPC `-32602` (`Invalid params`).
  - `-32601` (`Method not found`) is strictly reserved for unrecognized RPC methods (e.g. unknown JSON-RPC `method: "custom/unknown"`).
- **Current Implementation**:
  - `src/tools/registry.rs:296` returns `-32601` (`Method not found`) when a tool name is not found in the registry.
  - `src/prompts/registry.rs:271` returns `-32601` when a prompt is not found.
  - `src/resources/registry.rs:361, 488` returns `-32601` when a resource or resource template is not found.
- **Action Items**:
  - [x] Change not-found handler outcomes in `ToolRegistry`, `PromptRegistry`, and `ResourceRegistry` to return `JsonRpcErrorResponse::invalid_params` (`-32602`) instead of `method_not_found` (`-32601`).
  - [x] Reserve `method_not_found` (`-32601`) exclusively for `Router::dispatch` when the top-level JSON-RPC `method` string is unknown.

---

### 1.6 Schema Tag Serialization: `ResourceLink` Content Block

- **Spec Requirement**:
  - In `schema.ts:1720`: `export interface ResourceLink extends Resource { type: "resource_link"; }`.
  - Content block discriminator for resource links is `"resource_link"`.
- **Current Implementation**:
  - In `src/types/mcp/content.rs:79-86`, `ContentBlock` derives `#[serde(rename_all = "camelCase")]`, serializing `ContentBlock::ResourceLink` as `"type": "resourceLink"` instead of `"type": "resource_link"`.
- **Action Items**:
  - [x] Add `#[serde(rename = "resource_link")]` to `ContentBlock::ResourceLink` in `src/types/mcp/content.rs`.
  - [x] Update unit tests in `src/types/mcp/content.rs` to verify `"type": "resource_link"`.

---

## Priority 2: Medium (Missing Specification Capabilities & MRTR)

### 2.1 Multi Round-Trip Requests (MRTR - SEP-2322)

- **Spec Requirement**:
  - All result types (`Result`, `CallToolResult`, `ListToolsResult`, `ReadResourceResult`, `ListResourcesResult`, `ListResourceTemplatesResult`, `ListPromptsResult`, `GetPromptResult`, `CompleteResult`, `ServerDiscoverResult`) inherit from `Result` with required `resultType: "complete" | "input_required"`.
  - When additional input is needed (e.g. sampling, elicitation, user confirmation), server returns `InputRequiredResult` with `resultType: "input_required"`, `inputRequests`, and/or `requestState`.
  - Client sends subsequent request with `inputResponses` and `requestState`.
- **Current Implementation**:
  - `CallToolResult`, `ListToolsResult`, etc. have `result_type: Option<String>` defaulting to `"complete"`, but `CompleteResult` in `src/types/mcp/completion/mod.rs` is missing `result_type` completely.
  - `InputRequiredResult`, `InputRequests`, `InputResponses`, and `InputResponseRequestParams` types are not yet defined in `src/types/mcp/`.
- **Action Items**:
  - [x] Add `result_type: Option<String>` (or a typed `ResultType` enum) to `CompleteResult` in `src/types/mcp/completion/mod.rs`.
  - [x] Define MRTR types: `InputRequiredResult`, `InputRequests`, `InputResponses`, `InputRequest`, and `InputResponse` in `src/types/mcp/core/mrtr.rs`.
  - [x] Add support for handlers to return `InputRequiredResult` to enable multi round-trip workflows (elicitation and sampling).

---

### 2.2 Standard MCP Error Codes & Typed Payloads

- **Spec Requirement**:
  - Partitioned MCP error code ranges in `schema.ts`:
    - `-32020`: `HEADER_MISMATCH`
    - `-32021`: `MISSING_REQUIRED_CLIENT_CAPABILITY` (with `requiredCapabilities: ClientCapabilities` in `data`)
    - `-32022`: `UNSUPPORTED_PROTOCOL_VERSION` (with `supported: string[]`, `requested: string` in `data`)
- **Current Implementation**:
  - `src/types/jsonrpc/error.rs` only defines standard JSON-RPC 2.0 codes (`-32700`, `-32600`, `-32601`, `-32602`, `-32603`) and generic `ServerError(i32)`.
- **Action Items**:
  - [x] Add standard MCP error constants to `src/types/mcp/core/error.rs`:
    - `HEADER_MISMATCH` (`-32020`)
    - `MISSING_REQUIRED_CLIENT_CAPABILITY` (`-32021`)
    - `UNSUPPORTED_PROTOCOL_VERSION` (`-32022`)
  - [x] Add typed helper constructors on `JsonRpcError` and `JsonRpcErrorResponse` for `header_mismatch`, `missing_required_client_capability`, and `unsupported_protocol_version`.

---

### 2.3 Custom Parameter Headers (`x-mcp-header` & `Mcp-Param-{Name}`)

- **Spec Requirement**:
  - In `server/tools.md` and `basic/transports/streamable-http.md`: Tool arguments can specify `x-mcp-header: true` in their JSON Schema. When calling the tool via HTTP POST, the client MUST supply `Mcp-Param-{Name}` matching the argument value.
  - Server MUST validate that `Mcp-Param-{Name}` headers match the arguments in the request body, returning `-32020` (`HeaderMismatch`) if mismatched.
- **Current Implementation**:
  - Tool input schemas are inspected for `"x-mcp-header": true` property annotations during tool registration.
  - `Mcp-Param-{Name}` HTTP headers are extracted, sentinel-decoded, and verified against request body arguments during `tools/call` dispatching.
  - Any missing required parameter headers or value mismatches return HTTP `400 Bad Request` with error code `-32020` (`HeaderMismatch`).
- **Action Items**:
  - [x] Inspect tool input schemas for `x-mcp-header: true` during tool validation.
  - [x] Verify corresponding `Mcp-Param-{Name}` HTTP headers against arguments.

---

### 2.4 DNS Rebinding Protection (`Origin` Header Validation)

- **Spec Requirement**:
  - "Servers MUST validate the `Origin` header on incoming HTTP requests to prevent DNS rebinding attacks. If the `Origin` is missing or does not match the server's expected origin(s), the server MUST respond with HTTP `403 Forbidden`." ([`streamable-http.md`](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/streamable-http.html#security))
- **Current Implementation**:
  - `ServerConfig` and `McpRouter` support configuring `allowed_origins(Vec<String>)`.
  - `src/router/service.rs` inspects incoming `Origin` headers against the configured allowlist with case-insensitivity, trailing slash tolerance, and wildcard (`"*"`) support.
  - Untrusted, blank, or invalid `Origin` headers are rejected immediately with HTTP `403 Forbidden`.
- **Action Items**:
  - [x] Add origin validation configuration to `RouterConfig` / `ServerConfig` (e.g. `allowed_origins: Vec<String>`).
  - [x] Return `403 Forbidden` when an untrusted `Origin` header is received.

---

### 2.5 Capabilities Schema Alignment

- **Spec Requirement**:
  - `ClientCapabilities` in `schema.ts:1018` includes `extensions?: Extensions` and `roots?: { listChanged?: boolean }`.
  - `ServerCapabilities` in `schema.ts:1056` includes `extensions?: Extensions`.
- **Current Implementation**:
  - `ClientCapabilities` and `ServerCapabilities` in `src/types/mcp/core/capabilities.rs` lack the `extensions` map and `roots` capability.
- **Action Items**:
  - [x] Add `pub roots: Option<RootsCapability>` and `pub extensions: Option<HashMap<String, Value>>` to `ClientCapabilities`.
  - [x] Add `pub extensions: Option<HashMap<String, Value>>` to `ServerCapabilities`.

---

## Priority 3: Low (Protocol Cleanup & Future Roadmaps)

### 3.1 `logging/setLevel` Deprecation (SEP-2577)

- **Spec Requirement**:
  - `logging/setLevel` was removed from the active MCP specification as of `2026-07-28` in favor of per-request log levels via `_meta["io.modelcontextprotocol/logLevel"]`. It remains temporarily in the deprecated features registry.
- **Current Implementation**:
  - `mcp-routing` currently supports both `logging/setLevel` and per-request `_meta["io.modelcontextprotocol/logLevel"]`.
- **Action Items**:
  - [ ] Mark `logging/setLevel` router method and types with `#[deprecated]` documentation indicating preference for per-request `_meta["io.modelcontextprotocol/logLevel"]`.

---

### 3.2 Statelessness & `Mcp-Session-Id` Cleanup (SEP-2567)

- **Spec Requirement**:
  - Protocol-level session management was removed in `2026-07-28` (SEP-2567). Servers SHOULD NOT require or mint protocol session IDs; transports are designed to be stateless.
- **Current Implementation**:
  - `src/router/service.rs` generates a UUID session ID for each request if none is present and attaches `mcp-session-id` response header to all responses.
- **Action Items**:
  - [ ] Make `Mcp-Session-Id` header handling purely optional/pass-through rather than generating or enforcing it as standard protocol state.

---

### 3.3 Stateless Subscriptions Stream (`subscriptions/listen` - SEP-2575)

- **Spec Requirement**:
  - `subscriptions/listen` replaces legacy `resources/subscribe`, `resources/unsubscribe`, and SSE GET endpoints with a unified POST channel returning a Server-Sent Events (SSE) notification stream (`toolsListChanged`, `promptsListChanged`, `resourcesListChanged`, `resourceSubscriptions`), correlated with `_meta["io.modelcontextprotocol/subscriptionId"]`.
- **Current Implementation**:
  - `mcp-routing` does not currently implement `subscriptions/listen` or long-lived SSE streaming responses in `src/router/`.
- **Action Items**:
  - [ ] Plan and design a `subscriptions/listen` endpoint handler in `mcp-routing` with SSE body streaming support.

---

## Compliance Checklist Summary

| Area | Status | Spec Ref | Remediation |
| :--- | :---: | :--- | :--- |
| **Notification Status** | ✅ Compliant | `streamable-http.md` | Returns `202 Accepted` with empty body |
| **Method Not Found HTTP Status** | ✅ Compliant | `streamable-http.md` | Returns `404 Not Found` for unknown RPC method (`-32601`) |
| **Protocol / Header Error Status** | ✅ Compliant | `streamable-http.md` | Returns `400 Bad Request` for `-32020`, `-32021`, `-32022`, `-32600`, `-32700` |
| **Not Found Error Codes** | ✅ Compliant | `schema.ts:430-432` | Returns `-32602` (`Invalid params`) for missing tools/prompts/resources |
| **Header Enforcement (`MCP-Protocol-Version`)** | ⚠️ Fix Required | `streamable-http.md` | Require & validate header on all POST requests |
| **Header Verification (`Mcp-Method`, `Mcp-Name`)** | ⚠️ Fix Required | `streamable-http.md` | Reject missing / mismatched headers with `-32020` |
| **Sentinel Encoding (`=?base64?...?=`)** | ✅ Compliant | `streamable-http.md` | Decode RFC 2047 sentinel values in headers |
| **`ResourceLink` Discriminator Tag** | ✅ Compliant | `schema.ts:1720` | Rename tag `"resourceLink"` -> `"resource_link"` |
| **`CompleteResult.resultType`** | ⚠️ Fix Required | `schema.ts:2644` | Add `result_type` field |
| **Standard MCP Error Codes (`-32020..-32022`)** | ✅ Compliant | `schema.ts:435-535` | Defined in `mcp::core::error` with typed constructors |
| **`Origin` Header Security** | ✅ Compliant | `streamable-http.md` | Validate `Origin` header to prevent DNS rebinding (`403 Forbidden`) |
| **Custom Header Params (`x-mcp-header`)** | ✅ Compliant | `streamable-http.md` | Support `Mcp-Param-{Name}` matching |
| **Multi Round-Trip Requests (MRTR)** | 💡 Missing Feature | `schema.ts:580-618` | Implement `InputRequiredResult` & retry params |
| **Capabilities Extensions & Roots** | ✅ Compliant | `schema.ts:1018,1056` | Add `extensions` & `roots` to capability structs |
| **`logging/setLevel` Sunset** | ℹ️ Modernization | SEP-2577 | Mark deprecated in favor of per-request `_meta` |
| **Sessionless Transport (`Mcp-Session-Id`)** | ℹ️ Modernization | SEP-2567 | Demote from mandatory protocol state |
| **`subscriptions/listen` Stream** | ℹ️ Future Roadmap | SEP-2575 | Implement long-lived SSE channel |
