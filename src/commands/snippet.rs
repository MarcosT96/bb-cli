//! Snippet commands (Bitbucket's analogue of `gh gist`).
//!
//! Snippets are workspace-scoped (`/snippets/{workspace}`), not repo-scoped.

use reqwest::Method;
use serde_json::{json, Value};

use crate::cli::{GlobalArgs, SnippetArgs, SnippetCmd};
use crate::client::Client;
use crate::error::Result;
use crate::output;
use crate::repo as repo_path_util;

pub fn run(args: SnippetArgs, global: &GlobalArgs) -> Result<()> {
    let client = Client::new(global.project.clone())?;
    match args.cmd.unwrap_or(SnippetCmd::List { workspace: None }) {
        SnippetCmd::List { workspace } => list(&client, global, workspace),
        SnippetCmd::View { workspace, id } => view(&client, &workspace, &id),
    }
}

fn list(client: &Client, global: &GlobalArgs, workspace: Option<String>) -> Result<()> {
    let workspace = match workspace {
        Some(w) => w,
        None => {
            let repo = repo_path_util::repo_path(global.project.as_deref())?;
            repo.split('/').next().unwrap_or_default().to_string()
        }
    };
    let response =
        client.request_value(Method::GET, &format!("/snippets/{workspace}"), None, false)?;
    let values = response
        .get("values")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if values.is_empty() {
        output::line("No snippets found.", "yellow");
        return Ok(());
    }
    for s in &values {
        output::print_value(&json!({
            "id": field(s, "id"),
            "title": field(s, "title"),
            "private": s.get("is_private").and_then(Value::as_bool).unwrap_or(false),
            "created": field(s, "created_on"),
        }));
        output::line("", "white");
    }
    Ok(())
}

fn view(client: &Client, workspace: &str, id: &str) -> Result<()> {
    let s = client.request_value(
        Method::GET,
        &format!("/snippets/{workspace}/{id}"),
        None,
        false,
    )?;
    output::print_value(&json!({
        "id": field(&s, "id"),
        "title": field(&s, "title"),
        "private": s.get("is_private").and_then(Value::as_bool).unwrap_or(false),
        "created": field(&s, "created_on"),
        "owner": s.get("owner").and_then(|o| o.get("display_name")).cloned().unwrap_or(Value::Null),
    }));
    Ok(())
}

fn field(value: &Value, key: &str) -> String {
    match value.get(key) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Null) | None => String::new(),
        Some(other) => other.to_string(),
    }
}
