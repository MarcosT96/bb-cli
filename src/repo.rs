//! Repository path resolution.
//!
//! Ports `getRepoPath()` from `helpers.php`: resolve `owner/repo` either from
//! the `--project` global flag (accepting `owner/repo`, an `https://bitbucket.org/…`
//! URL, or an `git@bitbucket.org:…` SSH URL) or, failing that, by parsing the
//! local git remote `origin`. Regex-free: the original PHP patterns are simple
//! enough to match by hand, avoiding a regex dependency.

use std::process::Command;

use crate::error::{AppError, Result};

/// Resolve `owner/repo` for the current invocation.
pub fn repo_path(project: Option<&str>) -> Result<String> {
    if let Some(project) = project {
        return parse_project(project);
    }
    let remote = git_remote_origin()?;
    parse_bitbucket_remote(&remote).ok_or_else(|| {
        AppError::Repo(
            "Cannot get repository info. Are you sure this is a bitbucket repository?".into(),
        )
    })
}

/// Parse the `--project` value in any of the three accepted forms.
fn parse_project(project: &str) -> Result<String> {
    let project = project.trim();

    // Plain `owner/repo` (exactly one slash, no scheme/host).
    if !project.contains("://") && !project.contains('@') {
        if let Some(path) = as_owner_repo(project) {
            return Ok(path);
        }
    }

    if let Some(path) = parse_bitbucket_remote(project) {
        return Ok(path);
    }

    Err(AppError::Repo(
        "Invalid repository format. Expected: \"owner/repo\" or \"https://bitbucket.org/owner/repo\"".into(),
    ))
}

/// Validate `owner/repo`: exactly one slash, both parts non-empty.
fn as_owner_repo(s: &str) -> Option<String> {
    let s = s.trim_end_matches('/');
    let parts: Vec<&str> = s.split('/').collect();
    if parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty() {
        Some(format!("{}/{}", parts[0], parts[1]))
    } else {
        None
    }
}

/// Extract `owner/repo` from an https or ssh bitbucket.org URL.
/// Handles `https://bitbucket.org/owner/repo(.git)?` and
/// `git@bitbucket.org:owner/repo.git`.
fn parse_bitbucket_remote(url: &str) -> Option<String> {
    let url = url.trim();
    let rest = {
        let idx = url.find("bitbucket.org")?;
        &url[idx + "bitbucket.org".len()..]
    };
    // The separator after the host is `/` (https) or `:` (ssh).
    let rest = rest.trim_start_matches([':', '/']);
    let rest = rest.trim_end_matches('/');
    let rest = rest.strip_suffix(".git").unwrap_or(rest);
    as_owner_repo(rest)
}

/// Shell out to git for the origin remote URL (ports the PHP `exec`).
fn git_remote_origin() -> Result<String> {
    let output = Command::new("git")
        .args(["config", "--get", "remote.origin.url"])
        .output()?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_owner_repo() {
        assert_eq!(parse_project("acme/widgets").unwrap(), "acme/widgets");
    }

    #[test]
    fn parses_https_url() {
        assert_eq!(
            parse_project("https://bitbucket.org/acme/widgets").unwrap(),
            "acme/widgets"
        );
        assert_eq!(
            parse_project("https://bitbucket.org/acme/widgets.git").unwrap(),
            "acme/widgets"
        );
    }

    #[test]
    fn parses_ssh_url() {
        assert_eq!(
            parse_project("git@bitbucket.org:acme/widgets.git").unwrap(),
            "acme/widgets"
        );
    }

    #[test]
    fn parses_ssh_remote() {
        assert_eq!(
            parse_bitbucket_remote("git@bitbucket.org:acme/widgets.git"),
            Some("acme/widgets".to_string())
        );
    }

    #[test]
    fn rejects_non_bitbucket() {
        assert!(parse_bitbucket_remote("git@github.com:acme/widgets.git").is_none());
    }

    #[test]
    fn rejects_bad_project() {
        assert!(parse_project("https://example.com/foo").is_err());
    }
}
