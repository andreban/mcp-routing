// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use http::Request;
use http_body_util::BodyExt;
use mcp_routing::{
    McpRouter, RegisteredResourceTemplates,
    types::mcp::{
        CacheScope, Implementation,
        resources::{ResourceAnnotations, ResourceTemplate},
    },
};
use tower::Service;

fn create_test_server() -> Implementation {
    Implementation::new("test-template-server", "1.0.0")
        .with_title("Test Template Server")
}

#[tokio::test]
async fn test_resource_templates_list_empty() {
    let mut router = McpRouter::new(create_test_server());

    let req_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "resources/templates/list"
    });

    let request = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("Content-Type", "application/json")
        .body(req_body.to_string())
        .unwrap();

    let response = router.call(request).await.unwrap();
    assert_eq!(response.status(), 200);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let resp_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(resp_json["jsonrpc"], "2.0");
    assert_eq!(resp_json["id"], 1.0);
    assert_eq!(
        resp_json["result"]["resourceTemplates"],
        serde_json::json!([])
    );
}

#[tokio::test]
async fn test_resource_templates_list_multiple_rich_templates() {
    let tmpl1 = ResourceTemplate::new("file:///{+path}", "File Explorer")
        .title("Local Files")
        .description("Read local files dynamically")
        .mime_type("text/plain")
        .annotations(ResourceAnnotations::new().priority(0.8));

    let tmpl2 = ResourceTemplate::new("postgres://{schema}/{table}", "Database Tables")
        .title("Postgres Tables")
        .description("Access table contents")
        .mime_type("application/json");

    let mut router = McpRouter::new(create_test_server())
        .register_resource_template(tmpl1, |uri: String| async move {
            format!("File content for {uri}")
        })
        .register_resource_template(tmpl2, |uri: String| async move {
            format!("Table content for {uri}")
        });

    let req_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 50,
        "method": "resources/templates/list"
    });

    let request = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("Content-Type", "application/json")
        .body(req_body.to_string())
        .unwrap();

    let response = router.call(request).await.unwrap();
    assert_eq!(response.status(), 200);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let resp_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(resp_json["id"], 50.0);
    let templates = resp_json["result"]["resourceTemplates"].as_array().unwrap();
    assert_eq!(templates.len(), 2);
    assert_eq!(templates[0]["uriTemplate"], "file:///{+path}");
    assert_eq!(templates[0]["name"], "File Explorer");
    assert_eq!(templates[0]["title"], "Local Files");
    assert_eq!(templates[0]["annotations"]["priority"], 0.8);

    assert_eq!(
        templates[1]["uriTemplate"],
        "postgres://{schema}/{table}"
    );
    assert_eq!(templates[1]["name"], "Database Tables");
}

#[tokio::test]
async fn test_resource_templates_dynamic_read_dispatching() {
    let tmpl = ResourceTemplate::new("file:///{+path}", "Files");
    let mut router = McpRouter::new(create_test_server()).register_resource_template(
        tmpl,
        |uri: String| async move {
            format!("Dynamically read file from {uri}")
        },
    );

    let req_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "resources/read",
        "params": {
            "uri": "file:///src/models/user.rs"
        }
    });

    let request = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("Content-Type", "application/json")
        .body(req_body.to_string())
        .unwrap();

    let response = router.call(request).await.unwrap();
    assert_eq!(response.status(), 200);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let resp_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(resp_json["id"], 1.0);
    assert_eq!(
        resp_json["result"]["contents"][0]["text"],
        "Dynamically read file from file:///src/models/user.rs"
    );
    assert_eq!(
        resp_json["result"]["contents"][0]["uri"],
        "file:///src/models/user.rs"
    );
}

#[tokio::test]
async fn test_resource_templates_list_caching_directives() {
    let mut router = McpRouter::new(create_test_server())
        .resource_templates_list_ttl(180_000)
        .resource_templates_list_cache_scope(CacheScope::Public);

    let request = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("Content-Type", "application/json")
        .header("Mcp-Method", "resources/templates/list")
        .body(serde_json::json!({ "jsonrpc": "2.0", "id": 1 }).to_string())
        .unwrap();

    let response = router.call(request).await.unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(
        response.headers().get("Cache-Control").unwrap().to_str().unwrap(),
        "public, max-age=180"
    );

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let resp_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(resp_json["id"], 1.0);
    assert_eq!(resp_json["result"]["ttlMs"], 180_000);
    assert_eq!(resp_json["result"]["cacheScope"], "public");
}

#[tokio::test]
async fn test_resource_templates_list_custom_handler_with_extractors() {
    let mut router = McpRouter::new(create_test_server())
        .register_resource_template(
            ResourceTemplate::new("doc://public/{id}", "Public Docs"),
            |uri: String| async move { Ok::<_, String>(uri) },
        )
        .register_resource_template(
            ResourceTemplate::new("doc://internal/{id}", "Internal Docs"),
            |uri: String| async move { Ok::<_, String>(uri) },
        )
        .resource_templates_list(
            |RegisteredResourceTemplates(templates): RegisteredResourceTemplates| async move {
                let filtered: Vec<ResourceTemplate> = templates
                    .into_iter()
                    .filter(|t| t.uri_template.contains("public"))
                    .collect();
                Ok::<_, String>(filtered)
            },
        );

    let request = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("Content-Type", "application/json")
        .header("Mcp-Method", "resources/templates/list")
        .body(serde_json::json!({ "jsonrpc": "2.0", "id": 1 }).to_string())
        .unwrap();

    let response = router.call(request).await.unwrap();
    assert_eq!(response.status(), 200);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let resp_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(resp_json["id"], 1.0);
    let templates = resp_json["result"]["resourceTemplates"].as_array().unwrap();
    assert_eq!(templates.len(), 1);
    assert_eq!(templates[0]["uriTemplate"], "doc://public/{id}");
}
