//! Authentication commands (ports `Actions/Auth.php`).
//!
//! `save` prompts for an Atlassian email + API token and writes them to the
//! config file; `show` prints the saved auth block. App passwords were removed
//! by Bitbucket, so the guidance now points at Atlassian API tokens with scopes.

use dialoguer::{Input, Password};

use crate::cli::{AuthArgs, AuthCmd, GlobalArgs};
use crate::config::{self, Auth, Config};
use crate::error::Result;
use crate::output;

pub fn run(args: AuthArgs, _global: &GlobalArgs) -> Result<()> {
    match args.cmd.unwrap_or(AuthCmd::Save) {
        AuthCmd::Save => save(),
        AuthCmd::Show => show(),
    }
}

fn save() -> Result<()> {
    output::line(
        "This requires an Atlassian API token created WITH SCOPES (not a plain token).",
        "yellow",
    );
    output::line(
        "At the link below, choose \"Create API token with scopes\", pick the Bitbucket app,",
        "yellow",
    );
    output::line(
        "and select the scopes you need (they do NOT imply each other):",
        "yellow",
    );
    output::line("  read:repository:bitbucket    branch, browse", "cyan");
    output::line(
        "  read:pullrequest:bitbucket   pr list/diff/show, pr-details",
        "cyan",
    );
    output::line(
        "  write:pullrequest:bitbucket  pr approve/merge/decline/create",
        "cyan",
    );
    output::line(
        "  read:pipeline:bitbucket      pipeline get/latest/wait",
        "cyan",
    );
    output::line("  write:pipeline:bitbucket     pipeline run/custom", "cyan");
    output::line(
        "  read:account                 pr create (resolves your user)",
        "cyan",
    );
    output::line(
        "https://id.atlassian.com/manage-profile/security/api-tokens",
        "green",
    );

    let email: String = Input::new()
        .with_prompt("Atlassian email")
        .interact_text()?;
    let api_token: String = Password::new().with_prompt("API token").interact()?;

    let mut config = config::load()?;
    config.auth = Some(Auth {
        email: Some(email),
        api_token: Some(api_token),
        username: None,
        app_password: None,
    });
    config::save(&config)?;

    output::line("Auth info saved.", "green");
    Ok(())
}

fn show() -> Result<()> {
    let config: Config = config::load()?;
    match config.auth {
        Some(auth) => {
            if let Some(email) = &auth.email {
                output::print_value(&serde_json::json!({ "email": email }));
            }
            if auth.api_token.is_some() {
                output::print_value(&serde_json::json!({ "apiToken": "********" }));
            }
            if auth.is_legacy() {
                output::line(
                    "This config uses the old app-password scheme. Run \"bb auth save\" to migrate.",
                    "yellow",
                );
            }
            if auth.email.is_none() && auth.api_token.is_none() && !auth.is_legacy() {
                output::line("No auth info saved. Run \"bb auth save\" first.", "yellow");
            }
        }
        None => output::line("No auth info saved. Run \"bb auth save\" first.", "yellow"),
    }
    Ok(())
}
