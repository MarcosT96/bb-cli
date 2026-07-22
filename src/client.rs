//! Bitbucket REST API client.
//!
//! Ports `Base::makeRequest`. A `Client` cannot be built without valid
//! new-scheme credentials, so authentication is enforced by construction
//! (replacing the PHP `checkAuth()` call at the top of every request). The
//! status/error handling ladder mirrors `Base.php:85-109` exactly:
//!   * 401                        → `AppError::Unauthorized`
//!   * 409                        → swallowed (allowed, e.g. merge conflicts)
//!   * other non-2xx              → print body, then `AppError::Status`
//!   * body with `type == error`  → `AppError::Api(error.message)`
//!   * body that isn't JSON       → returned raw (used by `pr diff`)

use base64::Engine;
use reqwest::blocking::Client as HttpClient;
use reqwest::Method;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::config;
use crate::error::{AppError, Result};
use crate::output;
use crate::repo;

const API_BASE: &str = "https://api.bitbucket.org/2.0";

pub struct Client {
    http: HttpClient,
    auth_header: String,
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
        let encoded = base64::engine::general_purpose::STANDARD.encode(format!("{email}:{token}"));
        Ok(Self {
            http: HttpClient::builder().build()?,
            auth_header: format!("Basic {encoded}"),
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
        let full_url = if repo_scoped {
            let repo_path = repo::repo_path(self.project.as_deref())?;
            format!("{API_BASE}/repositories/{repo_path}{url}")
        } else {
            format!("{API_BASE}{url}")
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
            // 409 is tolerated (matches the PHP allowlist).
            if status.as_u16() != 409 {
                output::line(&body, "white");
                return Err(AppError::Status(status.as_u16()));
            }
        }

        Ok(body)
    }
}
