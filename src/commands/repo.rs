//! Repository commands (Bitbucket has no analogue in the original PHP tool;
//! this is new coverage toward `gh repo` parity).
//!
//! `list`/`view`/`create`/`fork`/`delete` hit `/repositories/...` directly
//! (account-scoped, not repo-prefixed), while `clone` shells out to git using
//! the SSH clone URL from the repo's `links`.

use std::process::Command;

use reqwest::Method;
use serde_json::{json, Value};

use crate::cli::{GlobalArgs, RepoArgs, RepoCmd};
use crate::client::Client;
use crate::error::{AppError, Result};
use crate::output;
use crate::repo as repo_path_util;

pub fn run(args: RepoArgs, global: &GlobalArgs) -> Result<()> {
    let client = Client::new(global.project.clone())?;
    match args.cmd.unwrap_or(RepoCmd::List { workspace: None }) {
        RepoCmd::List { workspace } => list(&client, global, workspace),
        RepoCmd::View { repo } => view(&client, global, repo),
        RepoCmd::Create { full_name, public } => create(&client, &full_name, public),
        RepoCmd::Clone { repo } => clone(&client, &repo),
        RepoCmd::Fork { repo } => fork(&client, global, repo),
        RepoCmd::Delete { repo, yes } => delete(&client, &repo, yes),
    }
}

/// Resolve a `owner/repo` argument, falling back to the current repo context.
fn resolve_repo(global: &GlobalArgs, arg: Option<String>) -> Result<String> {
    match arg {
        Some(r) => Ok(r),
        None => repo_path_util::repo_path(global.project.as_deref()),
    }
}

fn list(client: &Client, global: &GlobalArgs, workspace: Option<String>) -> Result<()> {
    // Default to the current repo's workspace (the part before the slash).
    let workspace = match workspace {
        Some(w) => w,
        None => {
            let repo = repo_path_util::repo_path(global.project.as_deref())?;
            repo.split('/').next().unwrap_or_default().to_string()
        }
    };

    let response = client.request_value(
        Method::GET,
        &format!("/repositories/{workspace}"),
        None,
        false,
    )?;
    let values = response
        .get("values")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if values.is_empty() {
        output::line("No repositories found.", "yellow");
        return Ok(());
    }
    for r in &values {
        output::print_value(&json!({
            "name": field(r, "full_name"),
            "private": r.get("is_private").and_then(Value::as_bool).unwrap_or(false),
            "language": field(r, "language"),
            "updated": field(r, "updated_on"),
        }));
        output::line("", "white");
    }
    Ok(())
}

fn view(client: &Client, global: &GlobalArgs, repo: Option<String>) -> Result<()> {
    let repo = resolve_repo(global, repo)?;
    let r = client.request_value(Method::GET, &format!("/repositories/{repo}"), None, false)?;
    output::print_value(&json!({
        "name": field(&r, "full_name"),
        "description": field(&r, "description"),
        "private": r.get("is_private").and_then(Value::as_bool).unwrap_or(false),
        "language": field(&r, "language"),
        "size": r.get("size").cloned().unwrap_or(Value::Null),
        "created": field(&r, "created_on"),
        "updated": field(&r, "updated_on"),
        "link": r.get("links").and_then(|l| l.get("html")).and_then(|h| h.get("href")).cloned().unwrap_or(Value::Null),
    }));
    Ok(())
}

fn create(client: &Client, full_name: &str, public: bool) -> Result<()> {
    if !full_name.contains('/') {
        return Err(AppError::Usage(
            "Create needs owner/name, e.g. \"myworkspace/my-repo\".".into(),
        ));
    }
    let payload = json!({ "is_private": !public, "scm": "git" });
    let r = client.request_value(
        Method::POST,
        &format!("/repositories/{full_name}"),
        Some(&payload),
        false,
    )?;
    output::line(&format!("Created {}", field(&r, "full_name")), "green");
    if let Some(href) = r
        .get("links")
        .and_then(|l| l.get("html"))
        .and_then(|h| h.get("href"))
        .and_then(Value::as_str)
    {
        output::line(href, "cyan");
    }
    Ok(())
}

fn clone(client: &Client, repo: &str) -> Result<()> {
    let r = client.request_value(Method::GET, &format!("/repositories/{repo}"), None, false)?;
    // Prefer the SSH clone URL, falling back to HTTPS.
    let clone_url = r
        .get("links")
        .and_then(|l| l.get("clone"))
        .and_then(Value::as_array)
        .and_then(|arr| {
            let ssh = arr
                .iter()
                .find(|e| e.get("name").and_then(Value::as_str) == Some("ssh"));
            ssh.or_else(|| arr.first())
        })
        .and_then(|e| e.get("href"))
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::Api("No clone URL in repository response.".into()))?;

    output::line(&format!("Cloning {repo} ..."), "green");
    let status = Command::new("git").arg("clone").arg(clone_url).status()?;
    if !status.success() {
        return Err(AppError::Repo("git clone failed.".into()));
    }
    Ok(())
}

fn fork(client: &Client, global: &GlobalArgs, repo: Option<String>) -> Result<()> {
    let repo = resolve_repo(global, repo)?;
    let r = client.request_value(
        Method::POST,
        &format!("/repositories/{repo}/forks"),
        Some(&json!({})),
        false,
    )?;
    output::line(&format!("Forked to {}", field(&r, "full_name")), "green");
    Ok(())
}

fn delete(client: &Client, repo: &str, yes: bool) -> Result<()> {
    if !yes {
        use dialoguer::Confirm;
        let confirmed = Confirm::new()
            .with_prompt(format!("Delete {repo}? This cannot be undone."))
            .default(false)
            .interact()?;
        if !confirmed {
            output::line("Aborted.", "yellow");
            return Ok(());
        }
    }
    client.request_value(
        Method::DELETE,
        &format!("/repositories/{repo}"),
        None,
        false,
    )?;
    output::line(&format!("Deleted {repo}"), "green");
    Ok(())
}

fn field(value: &Value, key: &str) -> String {
    match value.get(key) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Null) | None => String::new(),
        Some(other) => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;

    fn client(server: &MockServer) -> Client {
        Client::with_base(&server.base_url(), "me@example.com", "tok", None).unwrap()
    }

    #[test]
    fn create_posts_private_by_default() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST)
                .path("/repositories/acme/new-repo")
                .json_body(json!({ "is_private": true, "scm": "git" }));
            then.status(200).json_body(json!({
                "full_name": "acme/new-repo",
                "links": { "html": { "href": "https://bitbucket.org/acme/new-repo" } }
            }));
        });
        create(&client(&server), "acme/new-repo", false).unwrap();
        mock.assert();
    }

    #[test]
    fn create_public_flips_is_private() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST)
                .path("/repositories/acme/pub-repo")
                .json_body(json!({ "is_private": false, "scm": "git" }));
            then.status(200)
                .json_body(json!({ "full_name": "acme/pub-repo" }));
        });
        create(&client(&server), "acme/pub-repo", true).unwrap();
        mock.assert();
    }

    #[test]
    fn create_without_slash_is_usage_error() {
        let server = MockServer::start();
        let err = create(&client(&server), "noslash", false).unwrap_err();
        assert!(matches!(err, AppError::Usage(_)));
    }

    #[test]
    fn delete_with_yes_sends_delete() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(DELETE).path("/repositories/acme/old-repo");
            then.status(204);
        });
        delete(&client(&server), "acme/old-repo", true).unwrap();
        mock.assert();
    }
}
