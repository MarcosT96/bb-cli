//! Browse commands (ports `Actions/Browse.php`).
//!
//! Builds the repository's web URL and either opens it in the default browser
//! (`browse`) or just prints it (`show`). No API call is made, so no auth is
//! required. The `open` crate replaces the PHP per-OS `start`/`open`/`xdg-open`
//! switch.

use crate::cli::{BrowseArgs, BrowseCmd, GlobalArgs};
use crate::error::Result;
use crate::output;
use crate::repo;

pub fn run(args: BrowseArgs, global: &GlobalArgs) -> Result<()> {
    let url = format!(
        "https://bitbucket.org/{}",
        repo::repo_path(global.project.as_deref())?
    );

    match args.cmd.unwrap_or(BrowseCmd::Browse) {
        BrowseCmd::Browse => {
            output::line(&url, "white");
            open::that(&url)?;
            Ok(())
        }
        BrowseCmd::Show => {
            output::line(&url, "white");
            Ok(())
        }
    }
}
