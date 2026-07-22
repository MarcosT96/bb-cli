//! SSH key commands (`gh ssh-key` analogue).
//!
//! Account-scoped via `/user/ssh-keys`. Requires an API token with the
//! appropriate account scope.

use reqwest::Method;
use serde_json::{json, Value};

use crate::cli::{GlobalArgs, KeyArgs, KeyCmd};
use crate::client::Client;
use crate::error::Result;
use crate::output;

pub fn run(args: KeyArgs, global: &GlobalArgs) -> Result<()> {
    let client = Client::new(global.project.clone())?;
    match args.cmd.unwrap_or(KeyCmd::List) {
        KeyCmd::List => list(&client),
        KeyCmd::Add { key, label } => add(&client, &key, label.as_deref()),
        KeyCmd::Delete { uuid } => delete(&client, &uuid),
    }
}

fn list(client: &Client) -> Result<()> {
    let response = client.request_value(Method::GET, "/user/ssh-keys", None, false)?;
    let values = response
        .get("values")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if values.is_empty() {
        output::line("No SSH keys found.", "yellow");
        return Ok(());
    }
    for k in &values {
        output::print_value(&json!({
            "uuid": field(k, "uuid"),
            "label": field(k, "label"),
            "created": field(k, "created_on"),
        }));
        output::line("", "white");
    }
    Ok(())
}

fn add(client: &Client, key: &str, label: Option<&str>) -> Result<()> {
    let mut payload = json!({ "key": key });
    if let Some(l) = label {
        payload["label"] = json!(l);
    }
    let k = client.request_value(Method::POST, "/user/ssh-keys", Some(&payload), false)?;
    output::line(&format!("Added SSH key {}", field(&k, "uuid")), "green");
    Ok(())
}

fn delete(client: &Client, uuid: &str) -> Result<()> {
    client.request_value(
        Method::DELETE,
        &format!("/user/ssh-keys/{uuid}"),
        None,
        false,
    )?;
    output::line(&format!("Deleted SSH key {uuid}"), "green");
    Ok(())
}

fn field(value: &Value, key: &str) -> String {
    match value.get(key) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Null) | None => String::new(),
        Some(other) => other.to_string(),
    }
}
