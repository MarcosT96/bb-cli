//! Branch commands (ports `Actions/Branch.php`).
//!
//! `list` fetches `/refs/branches`, follows pagination, and prints each branch
//! with its owner and last-updated time. `user` and `name` are thin filters
//! that delegate to `list`. This is also the Phase-1 end-to-end auth proof: a
//! single authenticated GET that always returns data for any repo.

use crate::cli::{BranchArgs, BranchCmd, GlobalArgs};
use crate::client::Client;
use crate::error::Result;
use crate::models::{Branch, Paginated};
use crate::output;

pub fn run(args: BranchArgs, global: &GlobalArgs) -> Result<()> {
    let client = Client::new(global.project.clone())?;
    match args.cmd.unwrap_or(BranchCmd::List {
        user: None,
        branch: None,
        page: 1,
    }) {
        BranchCmd::List { user, branch, page } => list(&client, user, branch, page),
        BranchCmd::User { user } => list(&client, Some(user), None, 1),
        BranchCmd::Name { branch } => list(&client, None, Some(branch), 1),
    }
}

fn list(
    client: &Client,
    user_filter: Option<String>,
    branch_filter: Option<String>,
    start_page: u32,
) -> Result<()> {
    let mut url = format!("/refs/branches?page={start_page}");
    let mut printed = 0usize;

    loop {
        let page: Paginated<Branch> = client.get_json(&url)?;

        for b in &page.values {
            if let Some(uf) = &user_filter {
                if !contains_ci(&b.owner(), uf) {
                    continue;
                }
            }
            if let Some(bf) = &branch_filter {
                if !contains_ci(&b.name, bf) {
                    continue;
                }
            }
            print_branch(b);
            printed += 1;
        }

        // Follow the `next` link (an absolute URL) until exhausted.
        match page.next {
            Some(next) => url = strip_api_base(&next),
            None => break,
        }
    }

    if printed == 0 {
        output::line("No branches found.", "yellow");
    }
    Ok(())
}

fn print_branch(b: &Branch) {
    let owner = b.owner();
    let updated = if b.date().is_empty() {
        String::new()
    } else {
        output::format_relative_timestamp(&b.date())
    };
    output::print_value(&serde_json::json!({
        "name": b.name,
        "owner": owner,
        "updated": updated,
    }));
    output::line("", "white");
}

/// Case-insensitive substring match (ports the PHP `stripos` filter).
fn contains_ci(haystack: &str, needle: &str) -> bool {
    haystack.to_lowercase().contains(&needle.to_lowercase())
}

/// The `next` link is a full URL; the client re-prefixes the API base and repo
/// path, so reduce it to the repo-relative suffix.
fn strip_api_base(next: &str) -> String {
    if let Some(idx) = next.find("/refs/branches") {
        next[idx..].to_string()
    } else {
        next.to_string()
    }
}
