//! Pull request details — comments (ports `Actions/PrDetails.php`).
//!
//! `show` fetches every page of a PR's comments (100-page safety cap),
//! partitions them into general vs inline by the presence of `inline.path`,
//! sorts each group chronologically by `created_on`, and — when `unresolved`
//! is set — keeps only unresolved inline comments (general comments are never
//! resolvable). Comments are handled as raw `serde_json::Value` because the
//! unresolved test hinges on whether the `resolution` key is *present*, a
//! distinction typed structs would blur.

use serde_json::Value;

use crate::cli::{GlobalArgs, PrDetailsArgs, PrDetailsCmd};
use crate::client::Client;
use crate::error::Result;
use crate::models::Paginated;
use crate::output;

pub fn run(args: PrDetailsArgs, global: &GlobalArgs) -> Result<()> {
    let client = Client::new(global.project.clone())?;
    let PrDetailsCmd::Show { pr_id, unresolved } = args.cmd.unwrap_or(PrDetailsCmd::Show {
        pr_id: 0,
        unresolved: false,
    });
    show(&client, pr_id, unresolved)
}

/// Print a PR's comments, general section then inline section.
pub fn show(client: &Client, pr_id: u32, unresolved: bool) -> Result<()> {
    let (general, inline) = fetch_all_comments(client, pr_id, unresolved)?;

    output::line(&format!("## Pull Request Comments (PR #{pr_id})"), "green");
    output::line("", "white");

    output::line("### General Comments", "yellow");
    if general.is_empty() {
        output::line("No general comments found.", "cyan");
    } else {
        for comment in &general {
            let c = format_general(comment);
            output::line(&format!("{} ({}):", c.author, c.timestamp), "green");
            output::line(&c.content, "white");
            output::line("", "white");
        }
    }

    output::line("### Inline Code Comments", "yellow");
    if inline.is_empty() {
        output::line("No inline comments found.", "cyan");
    } else {
        for comment in &inline {
            let c = format_inline(comment);
            output::line(&format!("File: {}:{}", c.file, c.line), "cyan");
            output::line(&format!("{} ({}):", c.author, c.timestamp), "green");
            output::line(&c.content, "white");
            output::line("", "white");
        }
    }
    Ok(())
}

/// Fetch and partition all comments. Returns `(general, inline)`.
fn fetch_all_comments(
    client: &Client,
    pr_id: u32,
    unresolved: bool,
) -> Result<(Vec<Value>, Vec<Value>)> {
    let mut general: Vec<Value> = Vec::new();
    let mut inline: Vec<Value> = Vec::new();
    let mut page = 1u32;

    // Safety limit: 100 pages × 100 = 10,000 comments (matches PHP).
    while page <= 100 {
        let url = format!("/pullrequests/{pr_id}/comments?pagelen=100&page={page}");
        let response: Paginated<Value> = client.get_json(&url)?;

        for comment in response.values {
            if inline_path(&comment).is_none() {
                general.push(comment);
            } else {
                inline.push(comment);
            }
        }

        if response.next.is_none() {
            break;
        }
        page += 1;
    }

    // Chronological sort by created_on (ISO-8601 strings compare lexically).
    // `sort_by_cached_key` computes each key once rather than on every compare.
    general.sort_by_cached_key(created_on);
    inline.sort_by_cached_key(created_on);

    // The unresolved filter applies to inline comments only.
    if unresolved {
        inline.retain(is_unresolved);
    }

    Ok((general, inline))
}

/// A comment is unresolved when it has no `resolution` key at all (a present
/// key, even an empty object, means resolved).
fn is_unresolved(comment: &Value) -> bool {
    !comment
        .as_object()
        .map(|o| o.contains_key("resolution"))
        .unwrap_or(false)
}

struct General {
    author: String,
    timestamp: String,
    content: String,
}

struct Inline {
    author: String,
    timestamp: String,
    content: String,
    file: String,
    line: String,
}

fn format_general(comment: &Value) -> General {
    let deleted = comment
        .get("deleted")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    General {
        author: display_name(comment, deleted),
        timestamp: timestamp(comment),
        content: if deleted {
            "[DELETED]".to_string()
        } else {
            content_raw(comment)
        },
    }
}

fn format_inline(comment: &Value) -> Inline {
    let deleted = comment
        .get("deleted")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let line = get_path(comment, &["inline", "to"])
        .filter(|v| !v.is_null())
        .or_else(|| get_path(comment, &["inline", "from"]));
    Inline {
        author: display_name(comment, deleted),
        timestamp: timestamp(comment),
        content: if deleted {
            "[DELETED]".to_string()
        } else {
            content_raw(comment)
        },
        file: inline_path(comment).unwrap_or_default(),
        line: line
            .map(|v| match v {
                Value::Number(n) => n.to_string(),
                Value::String(s) => s.clone(),
                _ => String::new(),
            })
            .unwrap_or_default(),
    }
}

// --- small JSON accessors mirroring the PHP array_get dot-lookups ---

fn inline_path(comment: &Value) -> Option<String> {
    get_path(comment, &["inline", "path"])
        .and_then(|v| v.as_str().map(str::to_string))
        .filter(|s| !s.is_empty())
}

fn created_on(comment: &Value) -> String {
    comment
        .get("created_on")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn timestamp(comment: &Value) -> String {
    output::format_relative_timestamp(&created_on(comment))
}

fn content_raw(comment: &Value) -> String {
    get_path(comment, &["content", "raw"])
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
}

/// `user.display_name`, falling back to "Unknown" for deleted comments.
fn display_name(comment: &Value, deleted: bool) -> String {
    let name =
        get_path(comment, &["user", "display_name"]).and_then(|v| v.as_str().map(str::to_string));
    match name {
        Some(n) => n,
        None if deleted => "Unknown".to_string(),
        None => String::new(),
    }
}

fn get_path<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for key in path {
        current = current.get(key)?;
    }
    Some(current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn unresolved_when_no_resolution_key() {
        assert!(is_unresolved(&json!({"id": 1})));
    }

    #[test]
    fn resolved_when_resolution_key_present_even_empty() {
        assert!(!is_unresolved(&json!({"id": 1, "resolution": {}})));
        assert!(!is_unresolved(&json!({"id": 1, "resolution": null})));
    }

    #[test]
    fn deleted_comment_shows_placeholder_and_unknown() {
        let c = format_general(&json!({"deleted": true}));
        assert_eq!(c.content, "[DELETED]");
        assert_eq!(c.author, "Unknown");
    }

    #[test]
    fn inline_line_prefers_to_over_from() {
        let c = format_inline(&json!({
            "inline": {"path": "a.rs", "to": 42, "from": 7},
            "content": {"raw": "hi"}
        }));
        assert_eq!(c.line, "42");
        assert_eq!(c.file, "a.rs");
    }

    #[test]
    fn inline_line_falls_back_to_from() {
        let c = format_inline(&json!({
            "inline": {"path": "a.rs", "to": null, "from": 7},
            "content": {"raw": "hi"}
        }));
        assert_eq!(c.line, "7");
    }
}
