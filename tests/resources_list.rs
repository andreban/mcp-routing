// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

use http::Request;
use http_body_util::BodyExt;
use mcp_routing::{
    Extension, McpRouter, RegisteredResources,
    types::mcp::{
        CacheScope, Implementation, Role,
        resources::{ListResourcesResult, Resource, ResourceAnnotations},
    },
};
use tower::Service;

fn create_test_server() -> Implementation {
    Implementation::new("test-resource-server", "1.0.0")
        .with_title("Test Resource Server")
        .with_description("Server for testing resources capability")
}

#[tokio::test]
async fn test_resources_list_empty() {
    let mut router = McpRouter::new(create_test_server());

    let req_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "resources/list"
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
    assert_eq!(resp_json["result"]["resources"], serde_json::json!([]));
}

#[tokio::test]
async fn test_resources_list_multiple_rich_resources() {
    let res1 = Resource::new("file:///project/readme.md", "README")
        .title("Project Readme")
        .description("Overview documentation")
        .mime_type("text/markdown")
        .size(2048)
        .annotations(
            ResourceAnnotations::new()
                .audience(vec![Role::User, Role::Assistant])
                .priority(0.9)
                .last_modified("2026-08-15T10:00:00Z"),
        );

    let res2 = Resource::new("file:///project/config.json", "Config")
        .title("Project Configuration")
        .description("Runtime settings")
        .mime_type("application/json")
        .size(512);

    let mut router = McpRouter::new(create_test_server())
        .register_resource(res1, || async { "readme content" })
        .register_resource(res2, || async { "config content" });

    let req_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 100,
        "method": "resources/list"
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

    assert_eq!(resp_json["id"], 100.0);
    let resources = resp_json["result"]["resources"].as_array().unwrap();
    assert_eq!(resources.len(), 2);
    assert_eq!(resources[0]["uri"], "file:///project/readme.md");
    assert_eq!(resources[0]["name"], "README");
    assert_eq!(resources[0]["title"], "Project Readme");
    assert_eq!(resources[0]["size"], 2048);
    assert_eq!(resources[0]["annotations"]["priority"], 0.9);
    assert_eq!(
        resources[0]["annotations"]["lastModified"],
        "2026-08-15T10:00:00Z"
    );

    assert_eq!(resources[1]["uri"], "file:///project/config.json");
    assert_eq!(resources[1]["name"], "Config");
    assert_eq!(resources[1]["size"], 512);
}

#[tokio::test]
async fn test_resources_list_via_header_and_body_fallback() {
    let res = Resource::new("memo://insights", "Insights");
    let mut router =
        McpRouter::new(create_test_server()).register_resource(res, || async { "memo" });

    // Header method
    let req_header = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("Content-Type", "application/json")
        .header("Mcp-Method", "resources/list")
        .body(serde_json::json!({ "jsonrpc": "2.0", "id": 1 }).to_string())
        .unwrap();

    let resp_header = router.call(req_header).await.unwrap();
    assert_eq!(resp_header.status(), 200);
    let bytes = resp_header.into_body().collect().await.unwrap().to_bytes();
    let val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(val["id"], 1.0);
    assert_eq!(val["result"]["resources"][0]["uri"], "memo://insights");

    // Body fallback
    let req_body = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("Content-Type", "application/json")
        .body(
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "resources/list"
            })
            .to_string(),
        )
        .unwrap();

    let resp_body = router.call(req_body).await.unwrap();
    assert_eq!(resp_body.status(), 200);
    let bytes2 = resp_body.into_body().collect().await.unwrap().to_bytes();
    let val2: serde_json::Value = serde_json::from_slice(&bytes2).unwrap();
    assert_eq!(val2["id"], 2.0);
    assert_eq!(val2["result"]["resources"][0]["uri"], "memo://insights");
}

#[tokio::test]
async fn test_resources_capability_advertisement_in_discover() {
    let res = Resource::new("file:///data.csv", "Data");
    let mut router =
        McpRouter::new(create_test_server()).register_resource(res, || async { "data" });

    let request = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("Content-Type", "application/json")
        .header("Mcp-Method", "server/discover")
        .body(serde_json::json!({ "jsonrpc": "2.0", "id": 1 }).to_string())
        .unwrap();

    let response = router.call(request).await.unwrap();
    assert_eq!(response.status(), 200);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let resp_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert!(resp_json["result"]["capabilities"]["resources"].is_object());
}

#[tokio::test]
async fn test_resources_list_caching_directives() {
    let mut router = McpRouter::new(create_test_server())
        .resources_list_ttl(120_000)
        .resources_list_cache_scope(CacheScope::Public);

    let request = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("Content-Type", "application/json")
        .header("Mcp-Method", "resources/list")
        .body(serde_json::json!({ "jsonrpc": "2.0", "id": 1 }).to_string())
        .unwrap();

    let response = router.call(request).await.unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(
        response.headers().get("Cache-Control").unwrap().to_str().unwrap(),
        "public, max-age=120"
    );

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let resp_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(resp_json["result"]["ttlMs"], 120_000);
    assert_eq!(resp_json["result"]["cacheScope"], "public");
}

#[tokio::test]
async fn test_resources_list_custom_handler_with_extractors_and_filtering() {
    #[derive(Clone)]
    struct TenantId(String);

    let mut router = McpRouter::new(create_test_server())
        .register_resource(
            Resource::new("tenant://alpha/doc", "Alpha Doc"),
            || async { "alpha doc" },
        )
        .register_resource(
            Resource::new("tenant://beta/doc", "Beta Doc"),
            || async { "beta doc" },
        )
        .with_state(TenantId("alpha".to_string()))
        .resources_list(
            |Extension(tenant): Extension<TenantId>,
             RegisteredResources(registered): RegisteredResources| async move {
                let filtered: Vec<Resource> = registered
                    .into_iter()
                    .filter(|r| r.uri.contains(&tenant.0))
                    .collect();
                Ok::<_, String>(filtered)
            },
        );

    let request = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("Content-Type", "application/json")
        .header("Mcp-Method", "resources/list")
        .body(serde_json::json!({ "jsonrpc": "2.0", "id": 1 }).to_string())
        .unwrap();

    let response = router.call(request).await.unwrap();
    assert_eq!(response.status(), 200);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let resp_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(resp_json["id"], 1.0);
    let resources = resp_json["result"]["resources"].as_array().unwrap();
    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0]["uri"], "tenant://alpha/doc");
}

#[tokio::test]
async fn test_resources_list_custom_handler_with_pagination_cursor() {
    let mut router = McpRouter::new(create_test_server()).resources_list(
        |cursor: Option<String>| async move {
            if cursor.as_deref() == Some("page_2") {
                Ok::<_, String>(
                    ListResourcesResult::new(vec![Resource::new(
                        "file:///project/part2.txt",
                        "Part 2",
                    )])
                    .with_cache(Some(30_000), Some(CacheScope::Private)),
                )
            } else {
                Ok(
                    ListResourcesResult::new(vec![Resource::new(
                        "file:///project/part1.txt",
                        "Part 1",
                    )])
                    .with_next_cursor("page_2"),
                )
            }
        },
    );

    // First page
    let req1 = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("Content-Type", "application/json")
        .body(
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "resources/list"
            })
            .to_string(),
        )
        .unwrap();

    let resp1 = router.call(req1).await.unwrap();
    let bytes1 = resp1.into_body().collect().await.unwrap().to_bytes();
    let json1: serde_json::Value = serde_json::from_slice(&bytes1).unwrap();
    assert_eq!(json1["id"], 1.0);
    assert_eq!(json1["result"]["resources"][0]["name"], "Part 1");
    assert_eq!(json1["result"]["nextCursor"], "page_2");

    // Second page with cursor
    let req2 = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("Content-Type", "application/json")
        .body(
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "resources/list",
                "params": {
                    "cursor": "page_2"
                }
            })
            .to_string(),
        )
        .unwrap();

    let resp2 = router.call(req2).await.unwrap();
    let bytes2 = resp2.into_body().collect().await.unwrap().to_bytes();
    let json2: serde_json::Value = serde_json::from_slice(&bytes2).unwrap();
    assert_eq!(json2["id"], 2.0);
    assert_eq!(json2["result"]["resources"][0]["name"], "Part 2");
    assert!(json2["result"]["nextCursor"].is_null());
}

#[tokio::test]
async fn test_resources_list_custom_handler_error_propagation() {
    let mut router = McpRouter::new(create_test_server()).resources_list(|| async {
        Err::<Vec<Resource>, String>("Database connection failed".to_string())
    });

    let request = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("Content-Type", "application/json")
        .body(
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "resources/list"
            })
            .to_string(),
        )
        .unwrap();

    let response = router.call(request).await.unwrap();
    assert_eq!(response.status(), 200);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let resp_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(resp_json["id"], 1.0);
    assert_eq!(resp_json["error"]["code"], -32603);
    assert!(
        resp_json["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Database connection failed")
    );
}
