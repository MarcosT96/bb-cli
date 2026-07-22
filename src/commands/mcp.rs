//! MCP server (`bb mcp serve`) — exposes Bitbucket to AI assistants.
//!
//! This is the project's differentiator: nothing in the Bitbucket ecosystem
//! offers a Model Context Protocol server. It reuses the existing blocking
//! `Client`; since the MCP SDK (`rmcp`) is async, each tool runs the blocking
//! work inside `tokio::task::spawn_blocking`, so the client is reused verbatim
//! rather than rewritten.
//!
//! Tools come in three groups:
//! * read-only:  pr_list, pr_diff, pipeline_latest, branch_list
//! * generic:    bitbucket_api (authenticated passthrough)
//! * mutating:   pr_approve, pr_merge, pipeline_run — their descriptions flag
//!   them as destructive so MCP clients prompt before running.

use reqwest::Method;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_handler, tool_router, ServerHandler, ServiceExt};
use serde::Deserialize;
use serde_json::json;

use crate::cli::{GlobalArgs, McpArgs, McpCmd};
use crate::client::Client;
use crate::error::{AppError, Result as AppResult};

pub fn run(args: McpArgs, global: &GlobalArgs) -> AppResult<()> {
    match args.cmd.unwrap_or(McpCmd::Serve) {
        McpCmd::Serve => serve(global),
    }
}

/// Spin up a tokio runtime (only for this subcommand) and serve over stdio.
fn serve(global: &GlobalArgs) -> AppResult<()> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(AppError::Io)?;

    runtime.block_on(async move {
        let server = BitbucketMcp::new(global.project.clone());
        let service = server
            .serve((tokio::io::stdin(), tokio::io::stdout()))
            .await
            .map_err(|e| AppError::Api(format!("MCP serve error: {e}")))?;
        service
            .waiting()
            .await
            .map_err(|e| AppError::Api(format!("MCP wait error: {e}")))?;
        Ok::<(), AppError>(())
    })
}

#[derive(Clone)]
struct BitbucketMcp {
    /// The `--project` override, threaded into every tool's client.
    project: Option<String>,
    // Read by the `#[tool_handler]`-generated dispatch, which the dead-code
    // analysis can't see.
    #[allow(dead_code)]
    tool_router: rmcp::handler::server::router::tool::ToolRouter<Self>,
}

impl BitbucketMcp {
    fn new(project: Option<String>) -> Self {
        Self {
            project,
            tool_router: Self::tool_router(),
        }
    }
}

// ---- tool parameter types --------------------------------------------------

/// Optional repository override; falls back to the server's `--project` / git.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct RepoParam {
    /// Repository as "owner/repo". Optional if bb is run inside a git repo.
    repo: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct PrNumberParam {
    /// Repository as "owner/repo" (optional inside a git repo).
    repo: Option<String>,
    /// Pull request number.
    pr_number: u32,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct BranchParam {
    repo: Option<String>,
    /// Branch name.
    branch: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ApiParam {
    /// API endpoint, e.g. "/repositories/{repo}/pullrequests" or "user".
    endpoint: String,
    /// HTTP method (default GET).
    method: Option<String>,
    /// Optional JSON body for POST/PUT.
    body: Option<serde_json::Value>,
    /// Repository for {repo}/{workspace} placeholders (optional inside a repo).
    repo: Option<String>,
}

#[tool_router]
impl BitbucketMcp {
    // ---- read-only ---------------------------------------------------------

    #[tool(description = "List open pull requests for a repository (read-only).")]
    async fn pr_list(&self, Parameters(p): Parameters<RepoParam>) -> String {
        self.run_blocking(
            move |client| client.request_value(Method::GET, "/pullrequests?state=OPEN", None, true),
            p.repo,
        )
        .await
    }

    #[tool(description = "Get the raw unified diff of a pull request (read-only).")]
    async fn pr_diff(&self, Parameters(p): Parameters<PrNumberParam>) -> String {
        let n = p.pr_number;
        self.run_blocking_raw(
            move |client| client.request_raw(Method::GET, &format!("/pullrequests/{n}/diff")),
            p.repo,
        )
        .await
    }

    #[tool(description = "Show the latest pipeline for a repository (read-only).")]
    async fn pipeline_latest(&self, Parameters(p): Parameters<RepoParam>) -> String {
        self.run_blocking(
            move |client| {
                // Newest pipeline first, one item — then return that object.
                let coll = client.request_value(
                    Method::GET,
                    "/pipelines/?sort=-created_on&pagelen=1",
                    None,
                    true,
                )?;
                match coll.get("values").and_then(serde_json::Value::as_array) {
                    Some(values) if !values.is_empty() => Ok(values[0].clone()),
                    _ => Ok(serde_json::json!({ "message": "No pipelines found." })),
                }
            },
            p.repo,
        )
        .await
    }

    #[tool(description = "List branches for a repository (read-only).")]
    async fn branch_list(&self, Parameters(p): Parameters<RepoParam>) -> String {
        self.run_blocking(
            move |client| {
                client.request_value(Method::GET, "/refs/branches?pagelen=50", None, true)
            },
            p.repo,
        )
        .await
    }

    // ---- generic passthrough ----------------------------------------------

    #[tool(
        description = "Call any Bitbucket REST API endpoint (authenticated passthrough). \
                       Safe for GET. POTENTIALLY DESTRUCTIVE when method is POST/PUT/DELETE — \
                       those can modify or delete data (e.g. DELETE a repository). Confirm with \
                       the user before calling with any non-GET method."
    )]
    async fn bitbucket_api(&self, Parameters(p): Parameters<ApiParam>) -> String {
        let ApiParam {
            endpoint,
            method,
            body,
            repo,
        } = p;
        let method = method.unwrap_or_else(|| "GET".into());
        self.run_blocking(
            move |client| {
                let m = method
                    .to_uppercase()
                    .parse::<Method>()
                    .map_err(|_| AppError::Usage(format!("Invalid method '{method}'.")))?;
                // Substitute placeholders like the `bb api` command does.
                let ep = substitute(&endpoint, client)?;
                client.request_value(m, &ep, body.as_ref(), false)
            },
            repo,
        )
        .await
    }

    // ---- mutating (destructive) --------------------------------------------

    #[tool(
        description = "DESTRUCTIVE: approve a pull request. This changes review state on \
                       Bitbucket. Confirm with the user before calling."
    )]
    async fn pr_approve(&self, Parameters(p): Parameters<PrNumberParam>) -> String {
        let n = p.pr_number;
        self.run_blocking(
            move |client| {
                client.request_value(
                    Method::POST,
                    &format!("/pullrequests/{n}/approve"),
                    None,
                    true,
                )
            },
            p.repo,
        )
        .await
    }

    #[tool(
        description = "DESTRUCTIVE: merge a pull request. This is irreversible. Confirm with \
                       the user before calling."
    )]
    async fn pr_merge(&self, Parameters(p): Parameters<PrNumberParam>) -> String {
        let n = p.pr_number;
        self.run_blocking(
            move |client| {
                client.request_value(
                    Method::POST,
                    &format!("/pullrequests/{n}/merge"),
                    None,
                    true,
                )
            },
            p.repo,
        )
        .await
    }

    #[tool(
        description = "DESTRUCTIVE: run a pipeline on a branch. This triggers CI/CD. Confirm \
                       with the user before calling."
    )]
    async fn pipeline_run(&self, Parameters(p): Parameters<BranchParam>) -> String {
        let branch = p.branch;
        self.run_blocking(move |client| {
            let payload = json!({
                "target": { "ref_type": "branch", "type": "pipeline_ref_target", "ref_name": branch }
            });
            client.request_value(Method::POST, "/pipelines/", Some(&payload), true)
        }, p.repo)
        .await
    }

    // ---- helpers -----------------------------------------------------------

    /// Run a blocking closure that returns JSON, formatting the result (or
    /// error) as a string for the MCP tool response.
    async fn run_blocking<F>(&self, f: F, repo: Option<String>) -> String
    where
        F: FnOnce(&Client) -> AppResult<serde_json::Value> + Send + 'static,
    {
        let project = repo.or_else(|| self.project.clone());
        let joined = tokio::task::spawn_blocking(move || {
            let client = Client::new(project)?;
            let value = f(&client)?;
            Ok::<String, AppError>(
                serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string()),
            )
        })
        .await;
        match joined {
            Ok(Ok(s)) => s,
            Ok(Err(e)) => format!("Error: {e}"),
            Err(e) => format!("Error: task failed: {e}"),
        }
    }

    /// Like `run_blocking` but for tools returning raw text (e.g. a diff).
    async fn run_blocking_raw<F>(&self, f: F, repo: Option<String>) -> String
    where
        F: FnOnce(&Client) -> AppResult<String> + Send + 'static,
    {
        let project = repo.or_else(|| self.project.clone());
        let joined = tokio::task::spawn_blocking(move || {
            let client = Client::new(project)?;
            f(&client)
        })
        .await;
        match joined {
            Ok(Ok(s)) => s,
            Ok(Err(e)) => format!("Error: {e}"),
            Err(e) => format!("Error: task failed: {e}"),
        }
    }
}

/// Substitute {repo}/{workspace} placeholders using the client's repo context.
fn substitute(endpoint: &str, client: &Client) -> AppResult<String> {
    let mut ep = endpoint.to_string();
    if ep.contains("{repo}") || ep.contains("{workspace}") {
        let repo = client.resolve_repo()?;
        let workspace = repo.split('/').next().unwrap_or("").to_string();
        ep = ep
            .replace("{repo}", &repo)
            .replace("{workspace}", &workspace);
    }
    if !ep.starts_with('/') {
        ep = format!("/{ep}");
    }
    if let Some(rest) = ep.strip_prefix("/2.0") {
        ep = if rest.is_empty() {
            "/".into()
        } else {
            rest.to_string()
        };
    }
    Ok(ep)
}

#[tool_handler(
    name = "bb-bitbucket",
    version = "0.3.0",
    instructions = "Bitbucket Cloud tools. Read-only tools are safe; tools whose description \
                    begins with DESTRUCTIVE modify data — confirm with the user first."
)]
impl ServerHandler for BitbucketMcp {}
