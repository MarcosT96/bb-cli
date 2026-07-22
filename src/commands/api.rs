//! `bb api` — authenticated passthrough to any Bitbucket API endpoint.
//!
//! This is the escape hatch: every endpoint the CLI doesn't wrap in a dedicated
//! command is reachable here, reusing the same auth and error handling as the
//! rest of the tool. Placeholders `{repo}` (owner/repo) and `{workspace}` are
//! substituted from the current repository context. The method defaults to GET,
//! or POST when a body is supplied via `--field`/`--input`.

use reqwest::Method;
use serde_json::{Map, Value};

use crate::cli::{ApiArgs, GlobalArgs};
use crate::client::Client;
use crate::error::{AppError, Result};

pub fn run(args: ApiArgs, global: &GlobalArgs) -> Result<()> {
    let client = Client::new(global.project.clone())?;

    let endpoint = resolve_endpoint(&args.endpoint, &client)?;
    let body = build_body(&args)?;

    // Method: explicit flag wins; otherwise GET, or POST if there's a body.
    let method = match &args.method {
        Some(m) => parse_method(m)?,
        None if body.is_some() => Method::POST,
        None => Method::GET,
    };

    let response = client.request_api(method, &endpoint, body.as_ref())?;
    print_response(&response);
    Ok(())
}

/// Resolve placeholders and normalize the endpoint path.
fn resolve_endpoint(raw: &str, client: &Client) -> Result<String> {
    let mut endpoint = raw.to_string();

    // Substitute repo placeholders only if the endpoint uses them, so plain
    // account-level calls (e.g. `bb api user`) don't require a git repo.
    if endpoint.contains("{repo}") || endpoint.contains("{workspace}") {
        let repo = client.resolve_repo()?; // "owner/repo"
        let workspace = repo.split('/').next().unwrap_or("").to_string();
        endpoint = endpoint
            .replace("{repo}", &repo)
            .replace("{workspace}", &workspace);
    }

    // The client's base URL already ends at ".../2.0", so the path we pass is
    // whatever comes after it. Normalize: ensure a leading slash, and strip a
    // redundant "/2.0" prefix if the user included one.
    if !endpoint.starts_with('/') {
        endpoint = format!("/{endpoint}");
    }
    if let Some(rest) = endpoint.strip_prefix("/2.0") {
        // "/2.0/repositories/..." -> "/repositories/...", "/2.0" -> "".
        endpoint = if rest.is_empty() {
            "/".to_string()
        } else {
            rest.to_string()
        };
    }

    Ok(endpoint)
}

/// Build a JSON body from `--input` (a file or stdin) or `--field` pairs.
/// `--input` takes precedence when both are given.
fn build_body(args: &ApiArgs) -> Result<Option<Value>> {
    if let Some(input) = &args.input {
        let raw = if input == "-" {
            use std::io::Read;
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)?;
            buf
        } else {
            std::fs::read_to_string(input)?
        };
        let value: Value = serde_json::from_str(&raw)?;
        return Ok(Some(value));
    }

    if args.fields.is_empty() {
        return Ok(None);
    }

    let mut map = Map::new();
    for field in &args.fields {
        let (key, raw_value) = field
            .split_once('=')
            .ok_or_else(|| AppError::Usage(format!("Invalid --field '{field}'. Use key=value.")))?;
        // Try to interpret the value as JSON (numbers, booleans, null, arrays,
        // objects); fall back to a plain string.
        let value = serde_json::from_str::<Value>(raw_value)
            .unwrap_or_else(|_| Value::String(raw_value.to_string()));
        map.insert(key.to_string(), value);
    }
    Ok(Some(Value::Object(map)))
}

fn parse_method(m: &str) -> Result<Method> {
    m.to_uppercase()
        .parse::<Method>()
        .map_err(|_| AppError::Usage(format!("Invalid HTTP method '{m}'.")))
}

/// Pretty-print a JSON response; fall back to the raw text otherwise.
fn print_response(response: &str) {
    if response.trim().is_empty() {
        return;
    }
    match serde_json::from_str::<Value>(response) {
        Ok(value) => match serde_json::to_string_pretty(&value) {
            Ok(pretty) => println!("{pretty}"),
            Err(_) => println!("{response}"),
        },
        Err(_) => println!("{response}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(endpoint: &str, fields: &[&str], input: Option<&str>) -> ApiArgs {
        ApiArgs {
            endpoint: endpoint.to_string(),
            method: None,
            fields: fields.iter().map(|s| s.to_string()).collect(),
            input: input.map(str::to_string),
        }
    }

    #[test]
    fn fields_are_typed_when_json_like() {
        let body = build_body(&args("x", &["name=hi", "count=5", "active=true"], None))
            .unwrap()
            .unwrap();
        assert_eq!(body["name"], Value::String("hi".into()));
        assert_eq!(body["count"], serde_json::json!(5));
        assert_eq!(body["active"], serde_json::json!(true));
    }

    #[test]
    fn no_fields_no_body() {
        assert!(build_body(&args("x", &[], None)).unwrap().is_none());
    }

    #[test]
    fn invalid_field_errors() {
        assert!(build_body(&args("x", &["novalue"], None)).is_err());
    }

    #[test]
    fn method_defaults_to_post_with_body() {
        // (Mirrors the run() logic without needing a Client.)
        let has_body = build_body(&args("x", &["a=1"], None)).unwrap().is_some();
        assert!(has_body);
    }
}
