// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Authentication and access control helpers for multi-tenant isolation.

use mcp_routing::BearerAuth;

use super::models::MovieDb;

/// Resolves the authenticated `user_id` from a [`BearerAuth`] token against the database registry.
///
/// Returns an error if the token is missing or not recognized.
pub fn resolve_user(auth: Option<&BearerAuth>, db: &MovieDb) -> Result<String, String> {
    let bearer = auth.ok_or_else(|| {
        "Authentication required: Please provide a valid Bearer token (e.g. 'Authorization: Bearer token_alice_secret')".to_string()
    })?;

    db.auth_tokens
        .get(bearer.token())
        .cloned()
        .ok_or_else(|| "Invalid or unknown authentication token".to_string())
}

/// Resolves the `user_id` if a valid token is provided, or returns `None` for unauthenticated requests.
pub fn resolve_optional_user(auth: Option<&BearerAuth>, db: &MovieDb) -> Option<String> {
    auth.and_then(|b| db.auth_tokens.get(b.token()).cloned())
}

/// Verifies that the caller's authenticated identity matches the requested `target_user_id`.
///
/// Prevents Broken Object Level Authorization (IDOR) attacks across user resources.
pub fn verify_user_access(
    auth: Option<&BearerAuth>,
    target_user_id: &str,
    db: &MovieDb,
) -> Result<String, String> {
    let caller_id = resolve_user(auth, db)?;
    if caller_id != target_user_id {
        return Err(format!(
            "Access denied: User '{caller_id}' is not authorized to access data for user '{target_user_id}'"
        ));
    }
    Ok(caller_id)
}
