//! Bitbucket REST API client.
//!
//! Ports `Base::makeRequest`. A `Client` cannot be built without valid
//! new-scheme credentials, so authentication is enforced by construction
//! (replacing the PHP `checkAuth()` call at the top of every request). The
//! status/error handling ladder mirrors `Base.php:85-109`:
//! * 401 → `AppError::Unauthorized`
//! * 409 → swallowed (allowed, e.g. merge conflicts)
//! * other non-2xx → `AppError::Status(code, body)` (body carried in the error,
//!   never printed — printing to stdout would corrupt the MCP stream)
//! * body with `type == error` → `AppError::Api(error.message)`
//! * body that isn't JSON → returned raw (used by `pr diff`)

use base64::Engine;
use reqwest::blocking::Client as HttpClient;
use reqwest::Method;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::config;
use crate::error::{AppError, Result};
use crate::repo;

const API_BASE: &str = "https://api.bitbucket.org/2.0";

pub struct Client {
    http: HttpClient,
    auth_header: String,
    /// The API base URL (production by default; overridden in tests to point at
    /// a mock server).
    base: String,
    /// The `--project` override, if any, for repo-path resolution.
    project: Option<String>,
}

impl Client {
    /// Build a client from saved credentials. Errors with `NoAuth`/`LegacyAuth`
    /// when the config is missing or still on the app-password scheme.
    pub fn new(project: Option<String>) -> Result<Self> {
        let auth = config::require_auth()?;
        // `has_api_token()` guaranteed both are Some by `require_auth`.
        let email = auth.email.unwrap_or_default();
        let token = auth.api_token.unwrap_or_default();
        Self::with_base(API_BASE, &email, &token, project)
    }

    /// Build a client with an explicit base URL and credentials. Used by tests
    /// to target a mock server; production goes through [`Client::new`].
    pub fn with_base(
        base: &str,
        email: &str,
        token: &str,
        project: Option<String>,
    ) -> Result<Self> {
        let encoded = base64::engine::general_purpose::STANDARD.encode(format!("{email}:{token}"));
        Ok(Self {
            http: HttpClient::builder().build()?,
            auth_header: format!("Basic {encoded}"),
            base: base.to_string(),
            project,
        })
    }

    /// A repo-scoped GET returning typed JSON.
    pub fn get_json<T: DeserializeOwned>(&self, url: &str) -> Result<T> {
        let value = self.request_value(Method::GET, url, None, true)?;
        Ok(serde_json::from_value(value)?)
    }

    /// A repo-scoped request returning the parsed JSON value (untyped).
    pub fn request_value(
        &self,
        method: Method,
        url: &str,
        payload: Option<&Value>,
        repo_scoped: bool,
    ) -> Result<Value> {
        let text = self.send(method, url, payload, repo_scoped)?;
        // Empty body (e.g. a 409-swallowed action) → null.
        if text.trim().is_empty() {
            return Ok(Value::Null);
        }
        let value: Value = serde_json::from_str(&text)?;
        if value.get("type").and_then(Value::as_str) == Some("error") {
            let message = value
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(Value::as_str)
                .unwrap_or("Unknown API error")
                .to_string();
            return Err(AppError::Api(message));
        }
        Ok(value)
    }

    /// A repo-scoped request whose body may not be JSON (e.g. `pr diff`).
    /// Returns the raw response text.
    pub fn request_raw(&self, method: Method, url: &str) -> Result<String> {
        self.send(method, url, None, true)
    }

    /// A raw (non-repo-scoped) request to an arbitrary API path, returning the
    /// response body text verbatim. Powers `bb api`: the endpoint is used as
    /// given (after the caller resolves placeholders), the auth header is
    /// attached, and the status ladder still applies. The body is not parsed —
    /// callers decide whether to pretty-print it as JSON.
    pub fn request_api(
        &self,
        method: Method,
        endpoint: &str,
        payload: Option<&Value>,
    ) -> Result<String> {
        self.send(method, endpoint, payload, false)
    }

    /// The `--project`/git-resolved `owner/repo`, if resolvable. Used by
    /// `bb api` to substitute the `{repo}` / `{workspace}` placeholders.
    pub fn resolve_repo(&self) -> Result<String> {
        repo::repo_path(self.project.as_deref())
    }

    /// A repo-scoped mutation whose response we don't need.
    pub fn request_discard(
        &self,
        method: Method,
        url: &str,
        payload: Option<&Value>,
    ) -> Result<()> {
        self.request_value(method, url, payload, true)?;
        Ok(())
    }

    /// Core request: build URL (repo-prefixed when scoped), attach auth, send,
    /// and apply the status ladder. Returns the raw response body text.
    fn send(
        &self,
        method: Method,
        url: &str,
        payload: Option<&Value>,
        repo_scoped: bool,
    ) -> Result<String> {
        let base = &self.base;
        let full_url = if repo_scoped {
            let repo_path = repo::repo_path(self.project.as_deref())?;
            format!("{base}/repositories/{repo_path}{url}")
        } else {
            format!("{base}{url}")
        };

        let mut req = self
            .http
            .request(method.clone(), &full_url)
            .header("Content-Type", "application/json")
            .header("Authorization", &self.auth_header);

        if method != Method::GET {
            let body = payload
                .cloned()
                .unwrap_or(Value::Object(Default::default()));
            req = req.body(serde_json::to_string(&body)?);
        }

        let resp = req.send()?;
        let status = resp.status();
        let body = resp.text()?;

        if !status.is_success() {
            if status.as_u16() == 401 {
                return Err(AppError::Unauthorized);
            }
            // 409 is tolerated (matches the PHP allowlist). Carry the body in
            // the error rather than printing it — printing to stdout here would
            // corrupt the MCP JSON-RPC stream.
            if status.as_u16() != 409 {
                return Err(AppError::Status(status.as_u16(), body));
            }
        }

        Ok(body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use httpmock::prelude::*;
    use serde_json::json;

    /// A client pointed at a mock server, with fixed test credentials.
    fn test_client(server: &MockServer, project: Option<String>) -> Client {
        Client::with_base(&server.base_url(), "me@example.com", "tok123", project).unwrap()
    }

    #[test]
    fn sends_basic_auth_header_and_parses_json() {
        let server = MockServer::start();
        let expected = format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode("me@example.com:tok123")
        );
        let mock = server.mock(|when, then| {
            when.method(GET)
                .path("/user")
                .header("Authorization", &expected);
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({ "uuid": "{abc}" }));
        });

        let client = test_client(&server, None);
        let value = client
            .request_value(Method::GET, "/user", None, false)
            .unwrap();

        mock.assert();
        assert_eq!(value["uuid"], "{abc}");
    }

    #[test]
    fn maps_401_to_unauthorized() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/user");
            then.status(401);
        });
        let client = test_client(&server, None);
        let err = client
            .request_value(Method::GET, "/user", None, false)
            .unwrap_err();
        assert!(matches!(err, AppError::Unauthorized));
    }

    #[test]
    fn swallows_409() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/x");
            // 409 with an empty body → request succeeds, returns null.
            then.status(409);
        });
        let client = test_client(&server, None);
        let value = client
            .request_value(Method::POST, "/x", Some(&json!({})), false)
            .unwrap();
        assert!(value.is_null());
    }

    #[test]
    fn type_error_body_becomes_api_error() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/user");
            then.status(200).json_body(json!({
                "type": "error",
                "error": { "message": "boom" }
            }));
        });
        let client = test_client(&server, None);
        let err = client
            .request_value(Method::GET, "/user", None, false)
            .unwrap_err();
        assert!(matches!(err, AppError::Api(m) if m == "boom"));
    }

    #[test]
    fn other_status_carries_body() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/user");
            then.status(500).body("kaboom");
        });
        let client = test_client(&server, None);
        let err = client
            .request_value(Method::GET, "/user", None, false)
            .unwrap_err();
        match err {
            AppError::Status(code, body) => {
                assert_eq!(code, 500);
                assert!(body.contains("kaboom"));
            }
            other => panic!("expected Status, got {other:?}"),
        }
    }

    #[test]
    fn repo_scoped_url_is_prefixed() {
        let server = MockServer::start();
        // With --project set, repo path is "acme/widgets" and the request must
        // hit /repositories/acme/widgets/pullrequests.
        let mock = server.mock(|when, then| {
            when.method(GET)
                .path("/repositories/acme/widgets/pullrequests");
            then.status(200).json_body(json!({ "values": [] }));
        });
        let client = test_client(&server, Some("acme/widgets".to_string()));
        let value = client
            .request_value(Method::GET, "/pullrequests", None, true)
            .unwrap();
        mock.assert();
        assert_eq!(value["values"], json!([]));
    }

    #[test]
    fn post_sends_json_body() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST)
                .path("/x")
                .json_body(json!({ "title": "hi" }));
            then.status(201).json_body(json!({ "id": 1 }));
        });
        let client = test_client(&server, None);
        let value = client
            .request_value(Method::POST, "/x", Some(&json!({ "title": "hi" })), false)
            .unwrap();
        mock.assert();
        assert_eq!(value["id"], 1);
    }
}
