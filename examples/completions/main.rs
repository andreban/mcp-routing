// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # MCP Server with Autocompletions Capability Example
//!
//! Demonstrates registering completion handlers for:
//! 1. Prompt argument autocompletion (`ref/prompt`)
//! 2. Context-aware completions (inspecting other arguments in `CompleteContext`)
//! 3. Resource URI template parameter completion (`ref/resource`)
//! 4. Global fallback autocompletion provider
//! 5. Mounting into an Axum application

use std::error::Error;

use axum::Router;
use mcp_routing::{
    McpRouter,
    types::mcp::{
        CacheScope, Implementation,
        completion::{
            CompleteArgument, CompleteContext, CompleteParams, CompleteResult, Reference,
        },
        prompts::{Prompt, PromptArgument, PromptMessage, get::GetPromptResult},
        resources::ResourceTemplate,
    },
};
use serde::{Deserialize, Serialize};

/// Supported programming languages for the code review prompt.
const SUPPORTED_LANGUAGES: &[&str] = &[
    "rust",
    "python",
    "typescript",
    "javascript",
    "go",
    "cplusplus",
    "csharp",
    "ruby",
    "swift",
    "kotlin",
];

/// Database tables per schema for resource autocompletion.
fn get_schema_tables(schema: &str) -> &'static [&'static str] {
    match schema {
        "analytics" => &["daily_active_users", "page_views", "funnels", "retention_cohorts"],
        "production" => &["users", "accounts", "orders", "subscriptions", "payments"],
        "staging" => &["users_staging", "test_fixtures", "mock_orders"],
        _ => &["public_data", "system_logs"],
    }
}

// -----------------------------------------------------------------------------
// 1. Prompt Handlers & Completion Handlers
// -----------------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
struct CodeReviewParams {
    code: String,
    language: Option<String>,
    review_style: Option<String>,
}

async fn code_review_prompt(params: CodeReviewParams) -> Result<GetPromptResult, String> {
    let language = params.language.unwrap_or_else(|| "rust".to_string());
    let style = params.review_style.unwrap_or_else(|| "idiomatic".to_string());
    let prompt = format!(
        "Please review the following {language} code adhering to {style} guidelines:\n\n```\n{}\n```",
        params.code
    );
    Ok(GetPromptResult::new(vec![PromptMessage::user_text(prompt)]))
}

/// Autocompletion handler for the `language` argument of the `code_review` prompt.
async fn complete_review_language(arg: CompleteArgument) -> Vec<&'static str> {
    let prefix = arg.value.to_lowercase();
    SUPPORTED_LANGUAGES
        .iter()
        .copied()
        .filter(|lang| lang.starts_with(&prefix))
        .collect()
}

/// Context-aware autocompletion handler for the `review_style` argument.
/// Inspects the already-entered `language` parameter in `CompleteContext`.
async fn complete_review_style(
    arg: CompleteArgument,
    context: Option<CompleteContext>,
) -> Vec<&'static str> {
    let language = context
        .as_ref()
        .and_then(|ctx| ctx.get_argument("language"))
        .unwrap_or("general");

    let styles: &[&str] = match language {
        "rust" => &["idiomatic", "clippy-pedantic", "zero-copy-audit", "unsafe-free"],
        "python" => &["pep8-strict", "type-annotated", "black-formatting", "asyncio-audit"],
        "typescript" => &["strict-typescript", "functional-pure", "eslint-airbnb"],
        _ => &["general-cleanliness", "performance-focused", "security-audit"],
    };

    let prefix = arg.value.to_lowercase();
    styles
        .iter()
        .copied()
        .filter(|s| s.starts_with(&prefix))
        .collect()
}

// -----------------------------------------------------------------------------
// 2. Resource Template Completion Handlers
// -----------------------------------------------------------------------------

/// Autocompletion handler for table names in URI template `postgres://{schema}/{table}`.
async fn complete_database_table(
    arg: CompleteArgument,
    context: Option<CompleteContext>,
) -> Vec<String> {
    let schema = context
        .as_ref()
        .and_then(|c| c.get_argument("schema"))
        .unwrap_or("production");

    let prefix = arg.value.to_lowercase();
    get_schema_tables(schema)
        .iter()
        .filter(|tbl| tbl.starts_with(&prefix))
        .map(|tbl| (*tbl).to_string())
        .collect()
}

// -----------------------------------------------------------------------------
// 3. Fallback Completion Provider
// -----------------------------------------------------------------------------

/// Fallback provider for any unhandled autocompletion targets.
async fn fallback_completer(params: CompleteParams) -> CompleteResult {
    let target = match &params.reference {
        Reference::Prompt { name } => format!("prompt '{name}'"),
        Reference::Resource { uri } => format!("resource '{uri}'"),
    };

    tracing::info!(
        target = %target,
        arg_name = %params.argument.name,
        arg_value = %params.argument.value,
        "Serving fallback suggestion"
    );

    let prefix = params.argument.value;
    let values = vec![
        format!("{prefix}_suggestion_1"),
        format!("{prefix}_suggestion_2"),
    ];

    CompleteResult::new(values)
}

// -----------------------------------------------------------------------------
// Application Entrypoint
// -----------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let server_info = Implementation::new("completions-mcp-server", "1.0.0");

    // 1. Define prompt templates
    let code_review_def = Prompt::new("code_review")
        .title("Code Review Assistant")
        .description("Generates an expert code review prompt template")
        .argument(PromptArgument::new("code").description("Source code to review").required(true))
        .argument(PromptArgument::new("language").description("Programming language"))
        .argument(PromptArgument::new("review_style").description("Style guidelines"));

    // 2. Define resource templates
    let db_template = ResourceTemplate::new("postgres://{schema}/{table}", "Database Table Viewer")
        .description("Queries tabular database resources");

    // 3. Construct MCP router with completion handlers
    let mcp_router = McpRouter::new(server_info)
        // Prompt registration & argument completions
        .register_prompt(code_review_def, code_review_prompt)
        .register_prompt_arg_completion("code_review", "language", complete_review_language)
        .register_prompt_arg_completion("code_review", "review_style", complete_review_style)
        // Resource template registration & argument completion
        .register_resource_template(db_template, |uri: String| async move {
            format!("Table schema & records for: {uri}")
        })
        .register_resource_arg_completion(
            "postgres://{schema}/{table}",
            "table",
            complete_database_table,
        )
        // Global fallback autocompletion & caching
        .completion(fallback_completer)
        .completion_cache(Some(60000), Some(CacheScope::Public));

    // 4. Mount into an Axum application
    let app = Router::new().nest_service("/mcp", mcp_router);

    let addr = "127.0.0.1:3000";
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("🚀 MCP Completions server running at http://{addr}/mcp");
    println!("\nTry testing autocompletion with curl:");
    println!(
        r#"curl -X POST http://127.0.0.1:3000/mcp \
  -H "Content-Type: application/json" \
  -d '{{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "completion/complete",
    "params": {{
      "ref": {{ "type": "ref/prompt", "name": "code_review" }},
      "argument": {{ "name": "language", "value": "ru" }}
    }}
  }}'"#
    );

    axum::serve(listener, app).await?;
    Ok(())
}
