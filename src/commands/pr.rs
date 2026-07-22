//! Pull request lifecycle commands (ports `Actions/Pr.php`).
//!
//! Covers the full PR surface: read-only queries (list/diff/files/commits) and
//! mutations (approve/decline/merge/create/…). `create` reproduces the PHP
//! behavior precisely: current-branch source fallback via `git symbolic-ref`,
//! comma-split bulk creation, default reviewers minus the current user, and the
//! `--title`/`--description`/`-i` handling.

use std::process::Command;

use reqwest::Method;
use serde_json::{json, Value};

use crate::cli::{GlobalArgs, PrArgs, PrCmd};
use crate::client::Client;
use crate::error::{AppError, Result};
use crate::models::{Commit, CurrentUser, DiffstatRow, Paginated, PullRequest, PullRequestDetail};
use crate::output;

pub fn run(args: PrArgs, global: &GlobalArgs) -> Result<()> {
    let client = Client::new(global.project.clone())?;
    match args.cmd.unwrap_or(PrCmd::List { destination: None }) {
        PrCmd::List { destination } => list(&client, destination),
        PrCmd::Diff { pr_number } => diff(&client, pr_number),
        PrCmd::Files { pr_number } => files(&client, pr_number),
        PrCmd::Commits { pr_number } => commits(&client, pr_number),
        PrCmd::Approve { pr_numbers } => approve(&client, pr_numbers),
        PrCmd::NoApprove { pr_number } => simple(&client, Method::DELETE, pr_number, "approve"),
        PrCmd::RequestChanges { pr_number } => {
            simple(&client, Method::POST, pr_number, "request-changes")
        }
        PrCmd::NoRequestChanges { pr_number } => {
            simple(&client, Method::DELETE, pr_number, "request-changes")
        }
        PrCmd::Decline { pr_number } => decline(&client, pr_number),
        PrCmd::Merge { pr_number } => merge(&client, pr_number),
        PrCmd::Create {
            from_branch,
            to_branch,
            add_default_reviewers,
        } => create(
            &client,
            global,
            from_branch,
            to_branch,
            add_default_reviewers,
        ),
        PrCmd::Show { pr_id, unresolved } => {
            crate::commands::pr_details::show(&client, pr_id, unresolved)
        }
    }
}

fn list(client: &Client, destination: Option<String>) -> Result<()> {
    let page: Paginated<PullRequest> = client.get_json("/pullrequests?state=OPEN")?;

    for pr in &page.values {
        if let Some(dest) = &destination {
            if &pr.destination_branch() != dest {
                continue;
            }
        }

        let detail: PullRequestDetail = client.get_json(&format!("/pullrequests/{}", pr.id))?;

        let reviewers = detail
            .reviewers
            .iter()
            .filter_map(|r| r.display_name.clone())
            .collect::<Vec<_>>()
            .join(", ");

        let participants = detail
            .participants
            .iter()
            .filter_map(|p| match (&p.state, &p.user) {
                (Some(state), Some(user)) if !state.is_empty() => Some(format!(
                    "{} -> {}",
                    user.display_name.clone().unwrap_or_default(),
                    state
                )),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" | ");

        output::print_value(&json!({
            "id": pr.id,
            "author": pr.author_nickname(),
            "source": pr.source_branch(),
            "destination": pr.destination_branch(),
            "link": pr.html_link(),
            "reviewers": reviewers,
            "participants": participants,
        }));
        output::line("", "white");
    }
    Ok(())
}

fn diff(client: &Client, pr_number: u32) -> Result<()> {
    // Diff is raw unified-diff text, not JSON.
    let raw = client.request_raw(Method::GET, &format!("/pullrequests/{pr_number}/diff"))?;
    output::line(&raw, "yellow");
    Ok(())
}

fn files(client: &Client, pr_number: u32) -> Result<()> {
    let page: Paginated<DiffstatRow> =
        client.get_json(&format!("/pullrequests/{pr_number}/diffstat"))?;
    for row in &page.values {
        if let Some(path) = row.new.as_ref().and_then(|n| n.path.clone()) {
            output::line(&path, "yellow");
        }
    }
    Ok(())
}

fn commits(client: &Client, pr_number: u32) -> Result<()> {
    let page: Paginated<Commit> = client.get_json(&format!("/pullrequests/{pr_number}/commits"))?;
    for c in &page.values {
        if let Some(raw) = c.summary.as_ref().and_then(|s| s.raw.clone()) {
            // PHP replaced the literal "\n" sequence with real newlines.
            output::line(raw.replace("\\n", "\n").trim(), "yellow");
        }
    }
    Ok(())
}

fn approve(client: &Client, pr_numbers: Vec<u32>) -> Result<()> {
    if pr_numbers.is_empty() {
        return Err(AppError::Usage("Pr number required.".into()));
    }

    // First arg 0 → approve all open PRs.
    let targets: Vec<u32> = if pr_numbers[0] == 0 {
        let page: Paginated<PullRequest> = client.get_json("/pullrequests?state=OPEN")?;
        let ids: Vec<u32> = page.values.iter().map(|p| p.id as u32).collect();
        if ids.is_empty() {
            return Err(AppError::Usage("Pr not found.".into()));
        }
        ids
    } else {
        pr_numbers
    };

    for pr in targets {
        client.request_discard(Method::POST, &format!("/pullrequests/{pr}/approve"), None)?;
        output::line(&format!("{pr} Approved."), "green");
    }
    Ok(())
}

/// Shared handler for the approve/request-changes toggles that just hit an
/// endpoint and print the result (no-approve, request-changes, no-request-changes).
fn simple(client: &Client, method: Method, pr_number: u32, action: &str) -> Result<()> {
    let value = client.request_value(
        method,
        &format!("/pullrequests/{pr_number}/{action}"),
        None,
        true,
    )?;
    if !value.is_null() {
        output::print_value(&value);
    }
    Ok(())
}

fn decline(client: &Client, pr_number: u32) -> Result<()> {
    client.request_discard(
        Method::POST,
        &format!("/pullrequests/{pr_number}/decline"),
        None,
    )?;
    output::line("OK.", "green");
    Ok(())
}

fn merge(client: &Client, pr_number: u32) -> Result<()> {
    let value = client.request_value(
        Method::POST,
        &format!("/pullrequests/{pr_number}/merge"),
        None,
        true,
    )?;
    let state = value
        .get("state")
        .and_then(Value::as_str)
        .unwrap_or_default();
    output::line(state, "green");
    Ok(())
}

fn create(
    client: &Client,
    global: &GlobalArgs,
    from_branch: Option<String>,
    to_branch: Option<String>,
    add_default_reviewers: bool,
) -> Result<()> {
    let from_branch =
        from_branch.ok_or_else(|| AppError::Usage("A source branch is required.".into()))?;

    // When only one branch is given, it's the destination; source = current branch.
    let (source, dest) = match to_branch {
        Some(to) => (from_branch, to),
        None => (current_branch()?, from_branch),
    };

    let mut title = global.title.clone();
    let mut description = global.description.clone();

    if global.interactive {
        if title.is_none() {
            let t = prompt("PR title (leave empty for default):")?;
            title = if t.is_empty() { None } else { Some(t) };
        }
        if description.is_none() {
            let d = prompt("PR description (leave empty to skip):")?;
            description = if d.is_empty() { None } else { Some(d) };
        }
    }

    let reviewers = if add_default_reviewers {
        default_reviewers(client)?
    } else {
        Value::Array(vec![])
    };

    let mut responses = Vec::new();
    // Comma-split destination → bulk create.
    for dest in dest.split(',') {
        let dest = dest.trim();
        let mut payload = json!({
            "title": title.clone().unwrap_or_else(|| format!("Merge {source} into {dest}")),
            "source": { "branch": { "name": source } },
            "destination": { "branch": { "name": dest } },
            "reviewers": reviewers,
        });
        if let Some(desc) = &description {
            payload["description"] = json!(desc);
        }

        let response = client.request_value(Method::POST, "/pullrequests", Some(&payload), true)?;
        responses.push(json!({
            "id": response.get("id").cloned().unwrap_or(Value::Null),
            "link": response
                .get("links")
                .and_then(|l| l.get("html"))
                .and_then(|h| h.get("href"))
                .cloned()
                .unwrap_or(Value::Null),
        }));
    }

    output::print_value(&json!({ "pullRequests": responses }));
    Ok(())
}

/// Fetch default reviewers, excluding the current user (matched by uuid).
fn default_reviewers(client: &Client) -> Result<Value> {
    let current_uuid = current_user_uuid(client)?;
    let response = client.request_value(Method::GET, "/default-reviewers", None, true)?;
    let values = response
        .get("values")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let filtered: Vec<Value> = values
        .into_iter()
        .filter(|r| r.get("uuid").and_then(Value::as_str) != current_uuid.as_deref())
        .collect();
    Ok(Value::Array(filtered))
}

/// The `/user` endpoint is account-scoped, not repo-scoped.
fn current_user_uuid(client: &Client) -> Result<Option<String>> {
    let value = client.request_value(Method::GET, "/user", None, false)?;
    let user: CurrentUser = serde_json::from_value(value)?;
    Ok(user.uuid)
}

fn current_branch() -> Result<String> {
    let output = Command::new("git")
        .args(["symbolic-ref", "--short", "HEAD"])
        .output()?;
    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if branch.is_empty() {
        return Err(AppError::Repo(
            "Could not determine the current branch (git symbolic-ref failed).".into(),
        ));
    }
    Ok(branch)
}

fn prompt(question: &str) -> Result<String> {
    use dialoguer::Input;
    let input: String = Input::new()
        .with_prompt(question)
        .allow_empty(true)
        .interact_text()?;
    Ok(input)
}
