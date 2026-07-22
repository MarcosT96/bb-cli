//! Search commands (`gh search` analogue).
//!
//! Bitbucket's search surface is limited compared to GitHub — there's no public
//! code-search API — so this covers repository search within a workspace via
//! the collection query language (`?q=name~"..."`).

use reqwest::Method;
use serde_json::{json, Value};

use crate::cli::{GlobalArgs, SearchArgs, SearchCmd};
use crate::client::Client;
use crate::error::Result;
use crate::output;
use crate::repo as repo_path_util;

pub fn run(args: SearchArgs, global: &GlobalArgs) -> Result<()> {
    let client = Client::new(global.project.clone())?;
    match args.cmd {
        Some(SearchCmd::Repos { query, workspace }) => repos(&client, global, &query, workspace),
        None => {
            output::line(
                "Usage: bb search repos <query> [--workspace <ws>]",
                "yellow",
            );
            Ok(())
        }
    }
}

fn repos(
    client: &Client,
    global: &GlobalArgs,
    query: &str,
    workspace: Option<String>,
) -> Result<()> {
    let workspace = match workspace {
        Some(w) => w,
        None => {
            let repo = repo_path_util::repo_path(global.project.as_deref())?;
            repo.split('/').next().unwrap_or_default().to_string()
        }
    };

    // Bitbucket query language: name ~ "substring" (case-insensitive contains).
    let escaped = query.replace('"', "");
    let endpoint = format!("/repositories/{workspace}?q=name~\"{escaped}\"");
    let response = client.request_value(Method::GET, &endpoint, None, false)?;
    let values = response
        .get("values")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    if values.is_empty() {
        output::line(&format!("No repositories matching \"{query}\"."), "yellow");
        return Ok(());
    }
    for r in &values {
        output::print_value(&json!({
            "name": field(r, "full_name"),
            "language": field(r, "language"),
            "updated": field(r, "updated_on"),
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
