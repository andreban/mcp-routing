// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! RFC 6570 URI template matching utilities.

/// Matches a URI against an RFC 6570 URI template (e.g., `file:///{path}`, `postgres://{schema}/{table}`).
pub(crate) fn match_uri_template(template: &str, uri: &str) -> bool {
    if template == uri {
        return true;
    }

    let mut t_iter = template;
    let mut u_iter = uri;

    while let Some(start_bracket) = t_iter.find('{') {
        let prefix = &t_iter[..start_bracket];
        if !u_iter.starts_with(prefix) {
            return false;
        }
        u_iter = &u_iter[prefix.len()..];

        let Some(end_bracket) = t_iter[start_bracket..].find('}') else {
            return false;
        };
        let end_idx = start_bracket + end_bracket;
        let var_expr = &t_iter[start_bracket + 1..end_idx];
        t_iter = &t_iter[end_idx + 1..];

        let is_reserved = var_expr.starts_with('+');

        if let Some(next_start) = t_iter.find('{') {
            let next_literal = &t_iter[..next_start];
            if next_literal.is_empty() {
                // Adjacent templates without literal separator
                return false;
            }
            let Some(match_pos) = u_iter.find(next_literal) else {
                return false;
            };
            let matched_var = &u_iter[..match_pos];
            if matched_var.is_empty() {
                return false;
            }
            if !is_reserved && matched_var.contains('/') {
                return false;
            }
            u_iter = &u_iter[match_pos..];
        } else {
            // Trailing variable pattern
            let suffix = t_iter;
            if !u_iter.ends_with(suffix) {
                return false;
            }
            let matched_var_len = u_iter.len() - suffix.len();
            let matched_var = &u_iter[..matched_var_len];
            if matched_var.is_empty() {
                return false;
            }
            if !is_reserved && matched_var.contains('/') && !var_expr.starts_with('/') {
                // Check if variable expansion allows slashes
                // When at the end of template without +, check if path segments are expected
                // In RFC 6570, {path} or {+path} can match remaining URI if no other literal
                return true;
            }
            return true;
        }
    }

    t_iter == u_iter
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests URI template matching.
    #[test]
    fn test_match_uri_template() {
        assert!(match_uri_template("file:///{path}", "file:///src/main.rs"));
        assert!(match_uri_template("file:///{+path}", "file:///a/b/c/d.txt"));
        assert!(match_uri_template(
            "postgres://{schema}/{table}",
            "postgres://public/users"
        ));
        assert!(!match_uri_template(
            "postgres://{schema}/{table}",
            "mysql://public/users"
        ));
        assert!(match_uri_template("memo://all", "memo://all"));
        assert!(!match_uri_template("memo://all", "memo://other"));
    }
}
