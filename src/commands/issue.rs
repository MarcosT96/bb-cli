//! Issue commands (new coverage toward `gh issue` parity).
//!
//! Bitbucket Issues are an optional per-repository feature; if a repo has them
//! disabled the API returns 404, surfaced as a normal error. All endpoints are
//! repo-scoped (`/repositories/{repo}/issues`).

use reqwest::Method;
use serde_json::{json, Value};

use crate::cli::{GlobalArgs, IssueArgs, IssueCmd};
use crate::client::Client;
use crate::error::Result;
use crate::output;

pub fn run(args: IssueArgs, global: &GlobalArgs) -> Result<()> {
    let client = Client::new(global.project.clone())?;
    match args.cmd.unwrap_or(IssueCmd::List { state: None }) {
        IssueCmd::List { state } => list(&client, state),
        IssueCmd::View { id } => view(&client, id),
        IssueCmd::Create { title, body } => create(&client, &title, body.as_deref()),
        IssueCmd::Comment { id, body } => comment(&client, id, &body),
        IssueCmd::Close { id } => close(&client, id),
    }
}

fn list(client: &Client, state: Option<String>) -> Result<()> {
    let url = match &state {
        Some(s) => format!("/issues?q=state=\"{s}\""),
        None => "/issues".to_string(),
    };
    let response = client.request_value(Method::GET, &url, None, true)?;
    let values = response
        .get("values")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if values.is_empty() {
        output::line("No issues found.", "yellow");
        return Ok(());
    }
    for i in &values {
        output::print_value(&json!({
            "id": i.get("id").cloned().unwrap_or(Value::Null),
            "title": field(i, "title"),
            "state": field(i, "state"),
            "kind": field(i, "kind"),
            "priority": field(i, "priority"),
            "reporter": i.get("reporter").and_then(|r| r.get("display_name")).cloned().unwrap_or(Value::Null),
        }));
        output::line("", "white");
    }
    Ok(())
}

fn view(client: &Client, id: u32) -> Result<()> {
    let i = client.request_value(Method::GET, &format!("/issues/{id}"), None, true)?;
    output::print_value(&json!({
        "id": i.get("id").cloned().unwrap_or(Value::Null),
        "title": field(&i, "title"),
        "state": field(&i, "state"),
        "kind": field(&i, "kind"),
        "priority": field(&i, "priority"),
        "reporter": i.get("reporter").and_then(|r| r.get("display_name")).cloned().unwrap_or(Value::Null),
        "created": field(&i, "created_on"),
        "content": i.get("content").and_then(|c| c.get("raw")).cloned().unwrap_or(Value::Null),
    }));
    Ok(())
}

fn create(client: &Client, title: &str, body: Option<&str>) -> Result<()> {
    let mut payload = json!({ "title": title });
    if let Some(b) = body {
        payload["content"] = json!({ "raw": b });
    }
    let i = client.request_value(Method::POST, "/issues", Some(&payload), true)?;
    output::line(
        &format!(
            "Created issue #{}",
            i.get("id").and_then(Value::as_u64).unwrap_or(0)
        ),
        "green",
    );
    Ok(())
}

fn comment(client: &Client, id: u32, body: &str) -> Result<()> {
    let payload = json!({ "content": { "raw": body } });
    client.request_value(
        Method::POST,
        &format!("/issues/{id}/comments"),
        Some(&payload),
        true,
    )?;
    output::line("Comment added.", "green");
    Ok(())
}

fn close(client: &Client, id: u32) -> Result<()> {
    let payload = json!({ "state": "closed" });
    client.request_value(Method::PUT, &format!("/issues/{id}"), Some(&payload), true)?;
    output::line(&format!("Issue #{id} closed."), "green");
    Ok(())
}

fn field(value: &Value, key: &str) -> String {
    match value.get(key) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Null) | None => String::new(),
        Some(other) => other.to_string(),
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
    fn create_posts_title_and_body() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST)
                .path("/repositories/acme/widgets/issues")
                .json_body(json!({ "title": "Bug", "content": { "raw": "it broke" } }));
            then.status(201).json_body(json!({ "id": 5 }));
        });
        create(&client(&server), "Bug", Some("it broke")).unwrap();
        mock.assert();
    }

    #[test]
    fn create_without_body_omits_content() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST)
                .path("/repositories/acme/widgets/issues")
                .json_body(json!({ "title": "Just a title" }));
            then.status(201).json_body(json!({ "id": 6 }));
        });
        create(&client(&server), "Just a title", None).unwrap();
        mock.assert();
    }

    #[test]
    fn close_puts_state_closed() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(PUT)
                .path("/repositories/acme/widgets/issues/5")
                .json_body(json!({ "state": "closed" }));
            then.status(200).json_body(json!({ "id": 5 }));
        });
        close(&client(&server), 5).unwrap();
        mock.assert();
    }
}
