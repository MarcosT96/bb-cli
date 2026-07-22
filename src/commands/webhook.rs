//! Webhook commands (`gh` exposes these under repo settings; Bitbucket has a
//! first-class hooks API at `/repositories/{repo}/hooks`).

use reqwest::Method;
use serde_json::{json, Value};

use crate::cli::{GlobalArgs, WebhookArgs, WebhookCmd};
use crate::client::Client;
use crate::error::Result;
use crate::output;

pub fn run(args: WebhookArgs, global: &GlobalArgs) -> Result<()> {
    let client = Client::new(global.project.clone())?;
    match args.cmd.unwrap_or(WebhookCmd::List) {
        WebhookCmd::List => list(&client),
        WebhookCmd::Create { url, events } => create(&client, &url, events),
        WebhookCmd::Delete { uuid } => delete(&client, &uuid),
    }
}

fn list(client: &Client) -> Result<()> {
    let response = client.request_value(Method::GET, "/hooks", None, true)?;
    let values = response
        .get("values")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if values.is_empty() {
        output::line("No webhooks found.", "yellow");
        return Ok(());
    }
    for h in &values {
        output::print_value(&json!({
            "uuid": field(h, "uuid"),
            "url": field(h, "url"),
            "description": field(h, "description"),
            "active": h.get("active").and_then(Value::as_bool).unwrap_or(false),
            "events": h.get("events").cloned().unwrap_or(Value::Null),
        }));
        output::line("", "white");
    }
    Ok(())
}

fn create(client: &Client, url: &str, events: Vec<String>) -> Result<()> {
    let events = if events.is_empty() {
        vec!["repo:push".to_string()]
    } else {
        events
    };
    let payload = json!({
        "description": "Created by bb",
        "url": url,
        "active": true,
        "events": events,
    });
    let h = client.request_value(Method::POST, "/hooks", Some(&payload), true)?;
    output::line(&format!("Created webhook {}", field(&h, "uuid")), "green");
    Ok(())
}

fn delete(client: &Client, uuid: &str) -> Result<()> {
    client.request_value(Method::DELETE, &format!("/hooks/{uuid}"), None, true)?;
    output::line(&format!("Deleted webhook {uuid}"), "green");
    Ok(())
}

fn field(value: &Value, key: &str) -> String {
    match value.get(key) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Null) | None => String::new(),
        Some(other) => other.to_string(),
    }
}
