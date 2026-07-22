//! Command-line interface definition (clap derive).
//!
//! Ports the dispatch table and argv handling from the old `bin/bb` router.
//! Global flags (`--project`, `--title`, `--description`, `-i`) are marked
//! `global = true` so they can appear anywhere in the argument list, replacing
//! the hand-rolled argv scanner. Each command group exposes an optional
//! subcommand; when omitted, the group's default action runs (the PHP
//! `DEFAULT_METHOD`).

use clap::{Args, Parser, Subcommand};

/// Canonical list of built-in top-level command names. The single source of
/// truth used by the alias and extension dispatchers to avoid shadowing a real
/// command. Keep in sync with the `Command` enum below.
pub const BUILTIN_COMMANDS: &[&str] = &[
    "api",
    "alias",
    "auth",
    "branch",
    "browse",
    "env",
    "extension",
    "issue",
    "key",
    "pipeline",
    "pr",
    "pr-details",
    "repo",
    "search",
    "snippet",
    "upgrade",
    "webhook",
    "workspace",
    "help",
];

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
    /// Make an authenticated request to any Bitbucket API endpoint.
    Api(ApiArgs),
    /// Define command shortcuts.
    Alias(AliasArgs),
    /// Manage bb extensions (bb-<name> executables).
    Extension(ExtensionArgs),
    /// Run an MCP server exposing Bitbucket to AI assistants (stdio).
    Mcp(McpArgs),
    /// Authentication commands.
    Auth(AuthArgs),
    /// Branch commands.
    Branch(BranchArgs),
    /// Open or print the repository URL.
    Browse(BrowseArgs),
    /// Deployment environment commands.
    Env(EnvArgs),
    /// Issue commands.
    Issue(IssueArgs),
    /// Pipeline commands.
    Pipeline(PipelineArgs),
    /// Pull request commands.
    Pr(PrArgs),
    /// Repository commands.
    Repo(RepoArgs),
    /// Snippet commands.
    Snippet(SnippetArgs),
    /// Webhook commands.
    Webhook(WebhookArgs),
    /// SSH key commands.
    Key(KeyArgs),
    /// Search repositories.
    Search(SearchArgs),
    /// Workspace commands.
    Workspace(WorkspaceArgs),
    /// Pull request details (comments).
    #[command(name = "pr-details")]
    PrDetails(PrDetailsArgs),
    /// Upgrade bb to the latest release.
    Upgrade(UpgradeArgs),
}

// ---- api ------------------------------------------------------------------

#[derive(Debug, Args)]
pub struct ApiArgs {
    /// API endpoint, e.g. "/repositories/{workspace}/{repo}/pullrequests" or
    /// "user". A leading "/2.0" is added if absent. The placeholders {repo}
    /// (owner/repo) and {workspace} are substituted from the current repo.
    pub endpoint: String,

    /// HTTP method (GET by default; defaults to POST when --field is given).
    #[arg(short = 'X', long = "method")]
    pub method: Option<String>,

    /// Add a JSON body field as key=value (repeatable). Implies POST.
    #[arg(short = 'f', long = "field")]
    pub fields: Vec<String>,

    /// Read the request body from a JSON file (or "-" for stdin).
    #[arg(long)]
    pub input: Option<String>,

    /// Follow `next` links and output every page's values as one JSON array
    /// (GET collections only).
    #[arg(long)]
    pub paginate: bool,
}

// ---- alias ----------------------------------------------------------------

#[derive(Debug, Args)]
pub struct AliasArgs {
    #[command(subcommand)]
    pub cmd: Option<AliasCmd>,
}

#[derive(Debug, Subcommand)]
pub enum AliasCmd {
    /// List all aliases. Default action.
    #[command(visible_alias = "l")]
    List,
    /// Set an alias: bb alias set <name> "<expansion>". Use $1..$N for args.
    Set { name: String, expansion: String },
    /// Delete an alias.
    #[command(visible_alias = "rm")]
    Delete { name: String },
}

// ---- extension ------------------------------------------------------------

#[derive(Debug, Args)]
pub struct ExtensionArgs {
    #[command(subcommand)]
    pub cmd: Option<ExtensionCmd>,
}

#[derive(Debug, Subcommand)]
pub enum ExtensionCmd {
    /// List installed extensions. Default action.
    #[command(visible_alias = "l")]
    List,
    /// Install an extension by git URL or owner/repo (repo named bb-<name>).
    Install { source: String },
    /// Remove an installed extension by name.
    #[command(visible_alias = "rm")]
    Remove { name: String },
}

// ---- mcp ------------------------------------------------------------------

#[derive(Debug, Args)]
pub struct McpArgs {
    #[command(subcommand)]
    pub cmd: Option<McpCmd>,
}

#[derive(Debug, Subcommand)]
pub enum McpCmd {
    /// Serve the MCP server over stdio. Default action.
    Serve,
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
    /// Create an environment. Requires an API token with the
    /// `admin:pipeline:bitbucket` scope.
    #[command(name = "create", visible_alias = "n")]
    Create {
        name: String,
        /// Environment type: Test, Staging, or Production.
        #[arg(long, default_value = "Test")]
        env_type: String,
    },
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

// ---- issue ----------------------------------------------------------------

#[derive(Debug, Args)]
pub struct IssueArgs {
    #[command(subcommand)]
    pub cmd: Option<IssueCmd>,
}

#[derive(Debug, Subcommand)]
pub enum IssueCmd {
    /// List issues. Default action.
    #[command(visible_alias = "l")]
    List {
        /// Filter by state (new, open, resolved, closed, ...).
        #[arg(long)]
        state: Option<String>,
    },
    /// View an issue by id.
    #[command(visible_alias = "v")]
    View { id: u32 },
    /// Create an issue.
    Create {
        title: String,
        /// Issue body.
        #[arg(long)]
        body: Option<String>,
    },
    /// Comment on an issue.
    #[command(visible_alias = "c")]
    Comment { id: u32, body: String },
    /// Close (resolve) an issue.
    Close { id: u32 },
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

// ---- repo -----------------------------------------------------------------

#[derive(Debug, Args)]
pub struct RepoArgs {
    #[command(subcommand)]
    pub cmd: Option<RepoCmd>,
}

#[derive(Debug, Subcommand)]
pub enum RepoCmd {
    /// List repositories in a workspace. Default action.
    #[command(visible_alias = "l")]
    List {
        /// Workspace slug (defaults to the current repo's workspace).
        workspace: Option<String>,
    },
    /// View a repository's details.
    #[command(visible_alias = "v")]
    View {
        /// owner/repo (defaults to the current repo).
        repo: Option<String>,
    },
    /// Create a repository (owner/name).
    Create {
        full_name: String,
        /// Make the repository private (default: private).
        #[arg(long)]
        public: bool,
    },
    /// Clone a repository (owner/repo) via git.
    Clone { repo: String },
    /// Fork a repository (defaults to the current repo).
    Fork { repo: Option<String> },
    /// Delete a repository (owner/repo).
    Delete {
        repo: String,
        /// Skip the confirmation prompt.
        #[arg(long)]
        yes: bool,
    },
}

// ---- snippet --------------------------------------------------------------

#[derive(Debug, Args)]
pub struct SnippetArgs {
    #[command(subcommand)]
    pub cmd: Option<SnippetCmd>,
}

#[derive(Debug, Subcommand)]
pub enum SnippetCmd {
    /// List snippets in a workspace. Default action.
    #[command(visible_alias = "l")]
    List { workspace: Option<String> },
    /// View a snippet.
    #[command(visible_alias = "v")]
    View { workspace: String, id: String },
}

// ---- webhook --------------------------------------------------------------

#[derive(Debug, Args)]
pub struct WebhookArgs {
    #[command(subcommand)]
    pub cmd: Option<WebhookCmd>,
}

#[derive(Debug, Subcommand)]
pub enum WebhookCmd {
    /// List a repository's webhooks. Default action.
    #[command(visible_alias = "l")]
    List,
    /// Create a webhook.
    Create {
        url: String,
        /// Events to subscribe to (repeatable). Default: repo:push.
        #[arg(long = "event")]
        events: Vec<String>,
    },
    /// Delete a webhook by uuid.
    #[command(visible_alias = "rm")]
    Delete { uuid: String },
}

// ---- key ------------------------------------------------------------------

#[derive(Debug, Args)]
pub struct KeyArgs {
    #[command(subcommand)]
    pub cmd: Option<KeyCmd>,
}

#[derive(Debug, Subcommand)]
pub enum KeyCmd {
    /// List your account's SSH keys. Default action.
    #[command(visible_alias = "l")]
    List,
    /// Add an SSH key (from a public key string).
    Add {
        key: String,
        #[arg(long)]
        label: Option<String>,
    },
    /// Delete an SSH key by uuid.
    #[command(visible_alias = "rm")]
    Delete { uuid: String },
}

// ---- search ---------------------------------------------------------------

#[derive(Debug, Args)]
pub struct SearchArgs {
    #[command(subcommand)]
    pub cmd: Option<SearchCmd>,
}

#[derive(Debug, Subcommand)]
pub enum SearchCmd {
    /// Search repositories in a workspace by name substring. Default action.
    #[command(visible_alias = "r")]
    Repos {
        query: String,
        /// Workspace to search (defaults to the current repo's workspace).
        #[arg(long)]
        workspace: Option<String>,
    },
}

// ---- workspace ------------------------------------------------------------

#[derive(Debug, Args)]
pub struct WorkspaceArgs {
    #[command(subcommand)]
    pub cmd: Option<WorkspaceCmd>,
}

#[derive(Debug, Subcommand)]
pub enum WorkspaceCmd {
    /// List workspaces you belong to. Default action.
    #[command(visible_alias = "l")]
    List,
    /// List a workspace's projects.
    #[command(visible_alias = "p")]
    Projects { workspace: Option<String> },
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
