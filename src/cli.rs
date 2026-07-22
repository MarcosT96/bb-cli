//! Command-line interface definition (clap derive).
//!
//! Ports the dispatch table and argv handling from the old `bin/bb` router.
//! Global flags (`--project`, `--title`, `--description`, `-i`) are marked
//! `global = true` so they can appear anywhere in the argument list, replacing
//! the hand-rolled argv scanner. Each command group exposes an optional
//! subcommand; when omitted, the group's default action runs (the PHP
//! `DEFAULT_METHOD`).

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "bb", version, about = "Bitbucket Cloud CLI")]
pub struct Cli {
    #[command(flatten)]
    pub global: GlobalArgs,

    #[command(subcommand)]
    pub command: Command,
}

/// Flags accepted anywhere on the command line.
#[derive(Debug, Args, Clone, Default)]
pub struct GlobalArgs {
    /// Work with a repository, e.g. "owner/repo" or a full bitbucket.org URL.
    #[arg(long, global = true)]
    pub project: Option<String>,

    /// Pull request title (for `pr create`).
    #[arg(long, global = true)]
    pub title: Option<String>,

    /// Pull request description (for `pr create`).
    #[arg(long, global = true)]
    pub description: Option<String>,

    /// Interactive mode (prompt for missing input).
    #[arg(short = 'i', long, global = true)]
    pub interactive: bool,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Authentication commands.
    Auth(AuthArgs),
    /// Branch commands.
    Branch(BranchArgs),
    /// Open or print the repository URL.
    Browse(BrowseArgs),
    /// Deployment environment commands.
    Env(EnvArgs),
    /// Pipeline commands.
    Pipeline(PipelineArgs),
    /// Pull request commands.
    Pr(PrArgs),
    /// Pull request details (comments).
    #[command(name = "pr-details")]
    PrDetails(PrDetailsArgs),
    /// Upgrade bb to the latest release.
    Upgrade(UpgradeArgs),
}

// ---- auth -----------------------------------------------------------------

#[derive(Debug, Args)]
pub struct AuthArgs {
    #[command(subcommand)]
    pub cmd: Option<AuthCmd>,
}

#[derive(Debug, Subcommand)]
pub enum AuthCmd {
    /// Save auth info (Atlassian email + API token). Default action.
    Save,
    /// Show saved auth info.
    Show,
}

// ---- branch ---------------------------------------------------------------

#[derive(Debug, Args)]
pub struct BranchArgs {
    #[command(subcommand)]
    pub cmd: Option<BranchCmd>,
}

#[derive(Debug, Subcommand)]
pub enum BranchCmd {
    /// List branches. Default action.
    #[command(visible_alias = "l")]
    List {
        user: Option<String>,
        branch: Option<String>,
        #[arg(default_value_t = 1)]
        page: u32,
    },
    /// List branches by a user.
    #[command(visible_alias = "u")]
    User { user: String },
    /// List branches by name.
    #[command(visible_alias = "n")]
    Name { branch: String },
}

// ---- browse ---------------------------------------------------------------

#[derive(Debug, Args)]
pub struct BrowseArgs {
    #[command(subcommand)]
    pub cmd: Option<BrowseCmd>,
}

#[derive(Debug, Subcommand)]
pub enum BrowseCmd {
    /// Open the repository in a browser. Default action.
    #[command(visible_alias = "b")]
    Browse,
    /// Print the repository URL.
    #[command(name = "show", visible_alias = "url")]
    Show,
}

// ---- env ------------------------------------------------------------------

#[derive(Debug, Args)]
pub struct EnvArgs {
    #[command(subcommand)]
    pub cmd: Option<EnvCmd>,
}

#[derive(Debug, Subcommand)]
pub enum EnvCmd {
    /// List environments. Default action.
    #[command(visible_alias = "l")]
    List,
    /// List variables of an environment.
    #[command(visible_alias = "v")]
    Variables { env_uuid: String },
    /// Create an environment variable.
    #[command(name = "create-variable", visible_alias = "c")]
    CreateVariable {
        env_uuid: String,
        key: String,
        value: String,
        #[arg(default_value_t = false)]
        secured: bool,
    },
    /// Update an environment variable.
    #[command(name = "update-variable", visible_alias = "u")]
    UpdateVariable {
        env_uuid: String,
        var_uuid: String,
        key: String,
        value: String,
        #[arg(default_value_t = false)]
        secured: bool,
    },
}

// ---- pipeline -------------------------------------------------------------

#[derive(Debug, Args)]
pub struct PipelineArgs {
    #[command(subcommand)]
    pub cmd: Option<PipelineCmd>,
}

#[derive(Debug, Subcommand)]
pub enum PipelineCmd {
    /// Get a pipeline by number.
    Get { pipeline_number: u32 },
    /// Show the latest pipeline. Default action.
    Latest,
    /// Wait for a pipeline to complete.
    Wait { pipeline_number: Option<u32> },
    /// Run a pipeline on a branch.
    Run { branch: String },
    /// Run a custom pipeline on a branch.
    #[command(visible_alias = "c")]
    Custom { branch: String, pipeline: String },
}

// ---- pr -------------------------------------------------------------------

#[derive(Debug, Args)]
pub struct PrArgs {
    #[command(subcommand)]
    pub cmd: Option<PrCmd>,
}

#[derive(Debug, Subcommand)]
pub enum PrCmd {
    /// List open pull requests. Default action.
    #[command(visible_alias = "l")]
    List { destination: Option<String> },
    /// Show a pull request diff.
    #[command(visible_alias = "d")]
    Diff { pr_number: u32 },
    /// Show changed files of a pull request.
    Files { pr_number: u32 },
    /// Show commits of a pull request.
    #[command(visible_alias = "c")]
    Commits { pr_number: u32 },
    /// Approve pull requests (0 = all open).
    #[command(visible_alias = "a")]
    Approve { pr_numbers: Vec<u32> },
    /// Remove approval from a pull request.
    #[command(name = "no-approve", visible_alias = "na")]
    NoApprove { pr_number: u32 },
    /// Request changes on a pull request.
    #[command(name = "request-changes", visible_alias = "rc")]
    RequestChanges { pr_number: u32 },
    /// Remove a request-changes from a pull request.
    #[command(name = "no-request-changes", visible_alias = "nrc")]
    NoRequestChanges { pr_number: u32 },
    /// Decline a pull request.
    Decline { pr_number: u32 },
    /// Merge a pull request.
    #[command(visible_alias = "m")]
    Merge { pr_number: u32 },
    /// Create a pull request.
    Create {
        from_branch: Option<String>,
        to_branch: Option<String>,
        #[arg(default_value_t = true)]
        add_default_reviewers: bool,
    },
    /// Show a pull request's details (comments).
    Show {
        pr_id: u32,
        #[arg(default_value_t = false)]
        unresolved: bool,
    },
}

// ---- pr-details -----------------------------------------------------------

#[derive(Debug, Args)]
pub struct PrDetailsArgs {
    #[command(subcommand)]
    pub cmd: Option<PrDetailsCmd>,
}

#[derive(Debug, Subcommand)]
pub enum PrDetailsCmd {
    /// Show a pull request's comments. Default action.
    Show {
        pr_id: u32,
        #[arg(default_value_t = false)]
        unresolved: bool,
    },
}

// ---- upgrade --------------------------------------------------------------

#[derive(Debug, Args)]
pub struct UpgradeArgs {
    #[command(subcommand)]
    pub cmd: Option<UpgradeCmd>,
}

#[derive(Debug, Subcommand)]
pub enum UpgradeCmd {
    /// Download and install the latest release. Default action.
    Index,
}
