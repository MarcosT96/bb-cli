//! Typed application errors.
//!
//! Ports the error conditions the PHP tool keyed its UX on: authorization
//! failures, non-2xx HTTP statuses, Bitbucket `type: error` payloads, and the
//! "you must run `bb auth` first" case. `main` catches a single `AppError` and
//! prints it in red, mirroring the try/catch in the old `bin/bb` router.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    /// No auth configured yet — the user must run `bb auth save`.
    #[error("You have to configure auth info to use this command.\nRun \"bb auth save\" first.")]
    NoAuth,

    /// Config still uses the removed app-password scheme.
    #[error(
        "Your saved credentials use the old app-password scheme, which Bitbucket has removed.\n\
         Run \"bb auth save\" to re-authenticate with an Atlassian email + API token."
    )]
    LegacyAuth,

    /// HTTP 401 from Bitbucket.
    #[error(
        "Authorization error (HTTP 401).\n\
         Check your credentials — and make sure your API token was created WITH the required\n\
         Bitbucket scopes (a plain Atlassian token has none). Run \"bb auth\" for the scope list."
    )]
    Unauthorized,

    /// Bitbucket returned a `type: error` JSON body.
    #[error("{0}")]
    Api(String),

    /// Any other non-2xx (and non-409) status. Carries the status code and the
    /// response body so the caller can surface it (instead of the client
    /// printing to stdout, which would corrupt the MCP JSON-RPC stream).
    #[error("An error occurred, status code: {0}\n{1}")]
    Status(u16, String),

    /// Repo path could not be resolved.
    #[error("{0}")]
    Repo(String),

    /// A required argument was missing (ports the PHP "Too few arguments" case).
    #[error("{0}")]
    Usage(String),

    #[error(transparent)]
    Http(#[from] reqwest::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Prompt(#[from] dialoguer::Error),
}
