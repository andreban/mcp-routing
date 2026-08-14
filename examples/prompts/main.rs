// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! # MCP Server with Prompts Capability Example
//!
//! Demonstrates defining and registering prompt templates with:
//! 1. Prompt discovery (`prompts/list`)
//! 2. Parameterized prompt retrieval (`prompts/get`) with typed arguments
//! 3. Multi-turn prompt templates with `Role::User` and `Role::Assistant` messages
//! 4. Prompt caching directives (`Cache-Control` header propagation)

use std::error::Error;

use axum::Router;
use mcp_routing::{
    McpRouter,
    types::mcp::{
        CacheScope, Implementation,
        prompts::{Prompt, PromptArgument, PromptMessage, get::GetPromptResult},
    },
};
use serde::{Deserialize, Serialize};

/// Typed parameters for a code review prompt.
#[derive(Serialize, Deserialize)]
pub struct CodeReviewParams {
    pub code: String,
    pub language: Option<String>,
}

/// Prompt handler generating a structured code review prompt template.
async fn code_review_prompt(params: CodeReviewParams) -> Result<GetPromptResult, String> {
    if params.code.trim().is_empty() {
        return Err("Parameter 'code' cannot be empty".to_string());
    }

    let language = params.language.unwrap_or_else(|| "unspecified".to_string());
    let prompt_text = format!(
        "You are an expert software engineer and code reviewer.\n\
        Please review the following {language} code for correctness, security, and style:\n\n\
        ```{language}\n\
        {}\n\
        ```\n\n\
        Provide actionable feedback categorized by:\n\
        1. Correctness & Bugs\n\
        2. Performance & Efficiency\n\
        3. Security Considerations\n\
        4. Style & Readability",
        params.code
    );

    Ok(
        GetPromptResult::new(vec![PromptMessage::user_text(prompt_text)])
            .with_description("Structured code review template"),
    )
}

/// Typed parameters for a language tutor prompt.
#[derive(Serialize, Deserialize)]
pub struct TutorParams {
    pub language: String,
    pub level: Option<String>,
}

/// Prompt handler returning a multi-turn conversation starter.
async fn tutor_prompt(params: TutorParams) -> Result<Vec<PromptMessage>, String> {
    let level = params.level.unwrap_or_else(|| "beginner".to_string());

    Ok(vec![
        PromptMessage::user_text(format!(
            "I want to practice speaking {} at a {} level. Can you act as my tutor?",
            params.language, level
        )),
        PromptMessage::assistant_text(format!(
            "Certainly! I'd love to help you practice {}. We'll keep our conversation at a {} level. \
            To start, tell me about what you did today or what topics you'd like to explore!",
            params.language, level
        )),
    ])
}

/// Simple no-args prompt returning a static system prompt.
async fn system_prompt() -> &'static str {
    "You are a helpful, concise, and accurate AI assistant."
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let server_info = Implementation::new("prompts-mcp-server", "1.0.0");

    let code_review_def = Prompt::new("code_review")
        .title("Code Review Assistant")
        .description("Generates an expert code review prompt template for provided code")
        .argument(
            PromptArgument::new("code")
                .title("Source Code")
                .description("The code snippet or file content to review")
                .required(true),
        )
        .argument(
            PromptArgument::new("language")
                .title("Programming Language")
                .description("Programming language (e.g. rust, python, typescript)")
                .required(false),
        );

    let tutor_def = Prompt::new("language_tutor")
        .title("Language Tutor")
        .description("Interactive multi-turn conversation template for language learning")
        .argument(
            PromptArgument::new("language")
                .title("Target Language")
                .description("The language to practice")
                .required(true),
        )
        .argument(
            PromptArgument::new("level")
                .title("Proficiency Level")
                .description("Proficiency level (beginner, intermediate, advanced)")
                .required(false),
        );

    let system_def = Prompt::new("system_persona")
        .title("System Persona")
        .description("Standard concise assistant system prompt");

    let mcp_router = McpRouter::new(server_info)
        .instructions("MCP Server demonstrating Prompts capability (prompts/list and prompts/get)")
        // Configure prompts catalog caching (10 minutes, Public)
        .prompts_list_cache(Some(600_000), Some(CacheScope::Public))
        // Register parameterized prompt
        .register_prompt(code_review_def, code_review_prompt)
        // Register multi-turn prompt
        .register_prompt(tutor_def, tutor_prompt)
        // Register static prompt with specific caching directives (1 hour, Public)
        .register_prompt_with_cache(
            system_def,
            system_prompt,
            Some(3_600_000),
            Some(CacheScope::Public),
        );

    let app = Router::new().nest_service("/mcp", mcp_router);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    println!("Prompts MCP Server listening on http://127.0.0.1:3000/mcp");
    println!("  - prompts/list      -> lists available prompt templates (cached 10m)");
    println!("  - prompts/get       -> code_review, language_tutor, system_persona");

    axum::serve(listener, app).await?;
    Ok(())
}
