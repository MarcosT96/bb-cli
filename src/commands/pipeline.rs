//! Pipeline commands (ports `Actions/Pipeline.php`).
//!
//! `wait` polls a pipeline every 2s until it reaches `COMPLETED`; the PHP
//! version recursed with `sleep(2)`, ported here to an iterative loop to avoid
//! unbounded stack growth. `latest` derives the newest pipeline id from the
//! collection's `size` field, preserving the original (quirky) behavior.

use std::thread::sleep;
use std::time::Duration;

use reqwest::Method;
use serde_json::{json, Value};

use crate::cli::{GlobalArgs, PipelineArgs, PipelineCmd};
use crate::client::Client;
use crate::error::{AppError, Result};
use crate::output;
use crate::repo;

pub fn run(args: PipelineArgs, global: &GlobalArgs) -> Result<()> {
    let client = Client::new(global.project.clone())?;
    let project = global.project.clone();
    match args.cmd.unwrap_or(PipelineCmd::Latest) {
        PipelineCmd::Get { pipeline_number } => {
            print_pipeline(&client, pipeline_number, project.as_deref())
        }
        PipelineCmd::Latest => {
            let id = latest_pipeline_id(&client)?;
            print_pipeline(&client, id, project.as_deref())
        }
        PipelineCmd::Wait { pipeline_number } => wait(&client, pipeline_number, project.as_deref()),
        PipelineCmd::Run { branch } => run_pipeline(&client, &branch),
        PipelineCmd::Custom { branch, pipeline } => {
            custom(&client, &branch, &pipeline, project.as_deref())
        }
    }
}

/// Fetch a pipeline's raw JSON (`/pipelines/{n}`).
fn get_pipeline(client: &Client, number: u32) -> Result<Value> {
    client.request_value(Method::GET, &format!("/pipelines/{number}"), None, true)
}

fn print_pipeline(client: &Client, number: u32, project: Option<&str>) -> Result<()> {
    let response = get_pipeline(client, number)?;
    let repo_path = repo::repo_path(project)?;
    output::print_value(&json!({
        "id": number,
        "creator": path_str(&response, &["creator", "display_name"]),
        "repository": path_str(&response, &["repository", "name"]),
        "target": path_str(&response, &["target", "ref_name"]),
        "state": path_str(&response, &["state", "name"]),
        "stateResult": path_str(&response, &["state", "result", "name"]),
        "created": path_str(&response, &["created_on"]),
        "completed": path_str(&response, &["completed_on"]),
        "link": format!(
            "https://bitbucket.org/{repo_path}/addon/pipelines/home#!/results/{number}"
        ),
    }));
    Ok(())
}

fn wait(client: &Client, number: Option<u32>, project: Option<&str>) -> Result<()> {
    let number = match number {
        Some(n) => n,
        None => {
            let id = latest_pipeline_id(client)?;
            output::line(&format!("Pipeline: {id}"), "yellow");
            id
        }
    };

    // Safety cap so we never spin forever: 1800 polls × 2s ≈ 1 hour.
    const MAX_POLLS: u32 = 1800;

    for _ in 0..MAX_POLLS {
        let response = get_pipeline(client, number)?;
        let state = path_str(&response, &["state", "name"]);
        match state.as_str() {
            "COMPLETED" => {
                output::line("", "white");
                print_pipeline(client, number, project)?;
                return Ok(());
            }
            // A paused/halted pipeline will not reach COMPLETED on its own —
            // stop waiting and show its current state instead of hanging.
            "PAUSED" | "HALTED" => {
                output::line("", "white");
                output::line(
                    &format!("Pipeline is {state}; not waiting further."),
                    "yellow",
                );
                print_pipeline(client, number, project)?;
                return Ok(());
            }
            // Empty state means the id didn't resolve to a real pipeline.
            "" => {
                return Err(AppError::Api(format!(
                    "Pipeline {number} not found or has no state."
                )));
            }
            _ => {}
        }
        output::inline(".", "yellow");
        use std::io::Write;
        let _ = std::io::stdout().flush();
        sleep(Duration::from_secs(2));
    }

    Err(AppError::Api(format!(
        "Timed out waiting for pipeline {number} to complete."
    )))
}

fn run_pipeline(client: &Client, branch: &str) -> Result<()> {
    let payload = json!({
        "target": {
            "ref_type": "branch",
            "type": "pipeline_ref_target",
            "ref_name": branch,
        }
    });
    let response = client.request_value(Method::POST, "/pipelines/", Some(&payload), true)?;
    output::print_value(&response);
    Ok(())
}

fn custom(client: &Client, branch: &str, pipeline: &str, project: Option<&str>) -> Result<()> {
    let payload = json!({
        "target": {
            "ref_type": "branch",
            "type": "pipeline_ref_target",
            "ref_name": branch,
            "selector": { "type": "custom", "pattern": pipeline },
        }
    });
    let response = client.request_value(Method::POST, "/pipelines/", Some(&payload), true)?;
    let build_number = response
        .get("build_number")
        .and_then(Value::as_u64)
        .unwrap_or_default();
    let repo_path = repo::repo_path(project)?;
    output::print_value(&json!({
        "link": format!(
            "https://bitbucket.org/{repo_path}/addon/pipelines/home#!/results/{build_number}"
        ),
    }));
    Ok(())
}

/// The PHP code reads the collection `size` as the latest pipeline id.
fn latest_pipeline_id(client: &Client) -> Result<u32> {
    let response = client.request_value(Method::GET, "/pipelines/", None, true)?;
    Ok(response.get("size").and_then(Value::as_u64).unwrap_or(0) as u32)
}

fn path_str(value: &Value, path: &[&str]) -> String {
    let mut current = value;
    for key in path {
        match current.get(key) {
            Some(v) => current = v,
            None => return String::new(),
        }
    }
    match current {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;

    fn client(server: &MockServer) -> Client {
        Client::with_base(
            &server.base_url(),
            "me@example.com",
            "tok",
            Some("acme/widgets".to_string()),
        )
        .unwrap()
    }

    #[test]
    fn run_posts_branch_target() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST)
                .path("/repositories/acme/widgets/pipelines/")
                .json_body(json!({
                    "target": {
                        "ref_type": "branch",
                        "type": "pipeline_ref_target",
                        "ref_name": "main"
                    }
                }));
            then.status(201).json_body(json!({ "build_number": 42 }));
        });
        run_pipeline(&client(&server), "main").unwrap();
        mock.assert();
    }

    #[test]
    fn latest_id_reads_size_field() {
        // The `latest` id derivation reads the collection `size` (preserved quirk).
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(GET)
                .path("/repositories/acme/widgets/pipelines/");
            then.status(200)
                .json_body(json!({ "size": 3, "values": [] }));
        });
        let id = latest_pipeline_id(&client(&server)).unwrap();
        mock.assert();
        assert_eq!(id, 3);
    }
}
