//! Deployment environment commands (ports `Actions/Env.php`).
//!
//! Lists environments and their variables, and creates/updates variables. The
//! `secured` flag renders as "Yes"/"No", and mutation responses surface a
//! Bitbucket `error` object (message + detail) before exiting, matching the PHP.

use reqwest::Method;
use serde_json::{json, Value};

use crate::cli::{EnvArgs, EnvCmd, GlobalArgs};
use crate::client::Client;
use crate::error::{AppError, Result};
use crate::output;

pub fn run(args: EnvArgs, global: &GlobalArgs) -> Result<()> {
    let client = Client::new(global.project.clone())?;
    match args.cmd.unwrap_or(EnvCmd::List) {
        EnvCmd::List => list(&client),
        EnvCmd::Create { name, env_type } => create_environment(&client, &name, &env_type),
        EnvCmd::Variables { env_uuid } => variables(&client, &env_uuid),
        EnvCmd::CreateVariable {
            env_uuid,
            key,
            value,
            secured,
        } => create_variable(&client, &env_uuid, &key, &value, secured),
        EnvCmd::UpdateVariable {
            env_uuid,
            var_uuid,
            key,
            value,
            secured,
        } => update_variable(&client, &env_uuid, &var_uuid, &key, &value, secured),
    }
}

/// Create a deployment environment. The `environment_type` object needs a
/// `name`/`rank`/`type`; Bitbucket ranks Test=0, Staging=1, Production=2.
/// Requires an API token with the `admin:pipeline:bitbucket` scope.
fn create_environment(client: &Client, name: &str, env_type: &str) -> Result<()> {
    let rank = match env_type.to_lowercase().as_str() {
        "staging" => 1,
        "production" | "prod" => 2,
        _ => 0, // Test (default)
    };
    // Normalize the display name to Bitbucket's canonical casing.
    let type_name = match rank {
        1 => "Staging",
        2 => "Production",
        _ => "Test",
    };
    let payload = json!({
        "name": name,
        "environment_type": {
            "name": type_name,
            "rank": rank,
            "type": "deployment_environment_type"
        }
    });
    let env = client.request_value(Method::POST, "/environments", Some(&payload), true)?;
    output::line(
        &format!(
            "Created environment \"{}\" ({type_name})",
            field(&env, "name")
        ),
        "green",
    );
    if let Some(uuid) = env.get("uuid").and_then(Value::as_str) {
        output::line(uuid, "cyan");
    }
    Ok(())
}

fn list(client: &Client) -> Result<()> {
    let response = client.request_value(Method::GET, "/environments", None, true)?;
    for env in values(&response) {
        output::print_value(&json!({
            "uuid": field(env, "uuid"),
            "name": field(env, "name"),
        }));
        output::line("", "white");
    }
    Ok(())
}

fn variables(client: &Client, env_uuid: &str) -> Result<()> {
    let url = format!("/deployments_config/environments/{env_uuid}/variables");
    let response = client.request_value(Method::GET, &url, None, true)?;
    for var in values(&response) {
        output::print_value(&variable_view(var));
        output::line("", "white");
    }
    Ok(())
}

fn create_variable(
    client: &Client,
    env_uuid: &str,
    key: &str,
    value: &str,
    secured: bool,
) -> Result<()> {
    let url = format!("/deployments_config/environments/{env_uuid}/variables");
    let payload = json!({ "key": key, "value": value, "secured": secured });
    let response = client.request_value(Method::POST, &url, Some(&payload), true)?;
    variable_response(&response)
}

fn update_variable(
    client: &Client,
    env_uuid: &str,
    var_uuid: &str,
    key: &str,
    value: &str,
    secured: bool,
) -> Result<()> {
    let url = format!("/deployments_config/environments/{env_uuid}/variables/{var_uuid}");
    let payload = json!({ "key": key, "value": value, "secured": secured });
    let response = client.request_value(Method::PUT, &url, Some(&payload), true)?;
    variable_response(&response)
}

/// Print a mutation response, surfacing a Bitbucket `error` object first.
fn variable_response(response: &Value) -> Result<()> {
    if let Some(error) = response.get("error").filter(|e| !e.is_null()) {
        let message = error.get("message").and_then(Value::as_str).unwrap_or("");
        let detail = error.get("detail").and_then(Value::as_str).unwrap_or("");
        output::line(message, "yellow");
        output::line(detail, "red");
        return Err(AppError::Api(message.to_string()));
    }
    output::print_value(&variable_view(response));
    Ok(())
}

fn variable_view(var: &Value) -> Value {
    json!({
        "uuid": field(var, "uuid"),
        "key": field(var, "key"),
        "value": field(var, "value"),
        "secured": if var.get("secured").and_then(Value::as_bool).unwrap_or(false) {
            "Yes"
        } else {
            "No"
        },
    })
}

fn values(response: &Value) -> Vec<&Value> {
    response
        .get("values")
        .and_then(Value::as_array)
        .map(|a| a.iter().collect())
        .unwrap_or_default()
}

fn field(value: &Value, key: &str) -> String {
    match value.get(key) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Null) | None => String::new(),
        Some(other) => other.to_string(),
    }
}
