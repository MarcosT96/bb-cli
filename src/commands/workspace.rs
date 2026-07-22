//! Workspace commands (`gh org`-ish; Bitbucket's org unit is the workspace).
//!
//! `list` enumerates the workspaces the account belongs to; `projects` lists a
//! workspace's projects (Bitbucket's repo-grouping unit, shallower than GitHub
//! Projects). Both are account-scoped.

use reqwest::Method;
use serde_json::{json, Value};

use crate::cli::{GlobalArgs, WorkspaceArgs, WorkspaceCmd};
use crate::client::Client;
use crate::error::Result;
use crate::output;
use crate::repo as repo_path_util;

pub fn run(args: WorkspaceArgs, global: &GlobalArgs) -> Result<()> {
    let client = Client::new(global.project.clone())?;
    match args.cmd.unwrap_or(WorkspaceCmd::List) {
        WorkspaceCmd::List => list(&client),
        WorkspaceCmd::Projects { workspace } => projects(&client, global, workspace),
    }
}

fn list(client: &Client) -> Result<()> {
    let response = client.request_value(Method::GET, "/workspaces", None, false)?;
    let values = response
        .get("values")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if values.is_empty() {
        output::line("No workspaces found.", "yellow");
        return Ok(());
    }
    for w in &values {
        output::print_value(&json!({
            "slug": field(w, "slug"),
            "name": field(w, "name"),
            "private": w.get("is_private").and_then(Value::as_bool).unwrap_or(false),
        }));
        output::line("", "white");
    }
    Ok(())
}

fn projects(client: &Client, global: &GlobalArgs, workspace: Option<String>) -> Result<()> {
    let workspace = match workspace {
        Some(w) => w,
        None => {
            let repo = repo_path_util::repo_path(global.project.as_deref())?;
            repo.split('/').next().unwrap_or_default().to_string()
        }
    };
    let response = client.request_value(
        Method::GET,
        &format!("/workspaces/{workspace}/projects"),
        None,
        false,
    )?;
    let values = response
        .get("values")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if values.is_empty() {
        output::line("No projects found.", "yellow");
        return Ok(());
    }
    for p in &values {
        output::print_value(&json!({
            "key": field(p, "key"),
            "name": field(p, "name"),
            "private": p.get("is_private").and_then(Value::as_bool).unwrap_or(false),
        }));
        output::line("", "white");
    }
    Ok(())
}

fn field(value: &Value, key: &str) -> String {
    match value.get(key) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Null) | None => String::new(),
        Some(other) => other.to_string(),
    }
}
