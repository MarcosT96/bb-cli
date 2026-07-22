//! Self-update (ports `Actions/Upgrade.php`).
//!
//! Queries the GitHub releases API (unauthenticated — a separate bare HTTP
//! client with a User-Agent, never the Bitbucket auth client), compares the
//! running version against the latest tag with real semver ordering (the PHP
//! used a lexical string compare, which mis-ranks e.g. "1.0.10" < "1.0.9"),
//! and, if newer, downloads the release binary and atomically replaces the
//! running executable via `self_replace`.
//!
//! NOTE: the release asset is per-target, named `bb-<rustc-target-triple>`
//! (e.g. `bb-aarch64-apple-darwin`). The triple is captured at build time by
//! `build.rs` and read here via `env!("TARGET")`, so each build self-updates
//! using the asset that matches its own platform.

use std::io::Write;

use serde::Deserialize;

use crate::cli::{GlobalArgs, UpgradeArgs, UpgradeCmd};
use crate::error::{AppError, Result};
use crate::output;

const LATEST_RELEASE_URL: &str = "https://api.github.com/repos/MarcosT96/bb-cli/releases/latest";
const USER_AGENT: &str = "BB-Cli Curl Agent";
/// The rustc target triple this binary was built for (from build.rs). Names the
/// release asset for this platform: `bb-<TARGET>`.
const TARGET: &str = env!("TARGET");

#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
}

pub fn run(args: UpgradeArgs, _global: &GlobalArgs) -> Result<()> {
    match args.cmd.unwrap_or(UpgradeCmd::Index) {
        UpgradeCmd::Index => index(),
    }
}

fn index() -> Result<()> {
    let http = reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .build()?;

    let release: Release = http
        .get(LATEST_RELEASE_URL)
        .send()?
        .error_for_status()?
        .json()?;
    let latest_tag = release.tag_name.trim_start_matches('v').to_string();
    let current = env!("CARGO_PKG_VERSION");

    if is_newer(&latest_tag, current) {
        output::line(
            &format!("Fetching new version ({}) ...", release.tag_name),
            "green",
        );

        let download_url = format!(
            "https://github.com/MarcosT96/bb-cli/releases/download/{}/bb-{}",
            release.tag_name, TARGET
        );
        // Validate the response BEFORE writing anything: a 404 (e.g. no asset
        // built for this platform) returns an HTML error page, and writing that
        // over the running binary would brick the install.
        let response = http.get(&download_url).send()?;
        if !response.status().is_success() {
            return Err(AppError::Api(format!(
                "No release binary found for this platform ({TARGET}) at {download_url}"
            )));
        }
        let bytes = response.bytes()?;

        // Write the downloaded binary to a temp file next to nothing in
        // particular (a fresh temp dir), mark it executable, then let
        // `self_replace` atomically swap it in for the running executable.
        let temp_dir = tempfile::TempDir::new()?;
        let new_bin = temp_dir.path().join("bb");
        {
            let mut file = std::fs::File::create(&new_bin)?;
            file.write_all(&bytes)?;
            file.flush()?;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&new_bin, std::fs::Permissions::from_mode(0o755))?;
        }

        self_replace::self_replace(&new_bin)?;

        output::line("BB-CLI Updated", "green");
    } else {
        output::line("You are already on the latest version of bb-cli", "green");
    }
    Ok(())
}

/// Whether `latest` is strictly newer than `current`, by semver when both
/// parse, falling back to a string comparison otherwise.
fn is_newer(latest: &str, current: &str) -> bool {
    match (
        semver::Version::parse(latest),
        semver::Version::parse(current),
    ) {
        (Ok(l), Ok(c)) => l > c,
        _ => latest > current,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semver_ordering_beats_lexical() {
        // The bug the PHP had: lexically "1.0.10" < "1.0.9", but 1.0.10 IS newer.
        assert!(is_newer("1.0.10", "1.0.9"));
        assert!(!is_newer("1.0.9", "1.0.10"));
    }

    #[test]
    fn equal_is_not_newer() {
        assert!(!is_newer("1.2.3", "1.2.3"));
    }

    #[test]
    fn newer_major() {
        assert!(is_newer("2.0.0", "1.9.9"));
    }
}
