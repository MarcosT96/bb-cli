//! User config file handling.
//!
//! Reads and writes the same JSON file the PHP tool used
//! (`$HOME/.bitbucket-rest-cli-config.json`, from `config/app.php`). The auth
//! scheme changed: Bitbucket removed app passwords, so we now store an Atlassian
//! `email` + scoped `apiToken` (Basic auth `base64(email:apiToken)`) instead of
//! the old `username` + `appPassword`. Legacy keys are detected so we can tell
//! the user to re-authenticate rather than silently failing with a 401.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::{AppError, Result};

/// File name under `$HOME`, matching the PHP `userConfigFilePath`.
const CONFIG_FILE_NAME: &str = ".bitbucket-rest-cli-config.json";

/// The full config document. `extra` preserves any keys we don't model so a
/// round-trip write never drops unrelated config the user may have added.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<Auth>,

    /// User-defined command aliases: name -> expansion template.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub aliases: std::collections::BTreeMap<String, String>,

    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// Auth block. New keys are `email` + `apiToken`; the legacy `username` +
/// `appPassword` are accepted on read only, to detect old configs.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Auth {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(rename = "apiToken", skip_serializing_if = "Option::is_none")]
    pub api_token: Option<String>,

    // Legacy fields — read for detection, never written back.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(rename = "appPassword", skip_serializing_if = "Option::is_none")]
    pub app_password: Option<String>,
}

impl Auth {
    /// Whether both new-scheme credentials are present.
    pub fn has_api_token(&self) -> bool {
        self.email.is_some() && self.api_token.is_some()
    }

    /// Whether this is an old app-password config (and not yet migrated).
    pub fn is_legacy(&self) -> bool {
        !self.has_api_token() && (self.username.is_some() || self.app_password.is_some())
    }
}

/// Resolve the config file path (`$HOME/.bitbucket-rest-cli-config.json`).
pub fn path() -> Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| AppError::Repo("Could not resolve HOME directory.".into()))?;
    Ok(home.join(CONFIG_FILE_NAME))
}

/// Load the config, returning a default (empty) config if the file is absent.
pub fn load() -> Result<Config> {
    let path = path()?;
    match std::fs::read_to_string(&path) {
        Ok(contents) if contents.trim().is_empty() => Ok(Config::default()),
        Ok(contents) => Ok(serde_json::from_str(&contents)?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
        Err(e) => Err(e.into()),
    }
}

/// Write the config back with pretty formatting (matching PHP's JSON_PRETTY_PRINT).
///
/// The file holds the API token in plaintext, so on Unix it is restricted to
/// `0o600` (owner read/write only).
pub fn save(config: &Config) -> Result<()> {
    let path = path()?;
    let json = serde_json::to_string_pretty(config)?;
    std::fs::write(&path, json)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

/// Return the current auth, mapping "no config" and "legacy config" to their
/// respective errors. Used by the client before every authenticated request.
pub fn require_auth() -> Result<Auth> {
    let config = load()?;
    match config.auth {
        Some(auth) if auth.has_api_token() => Ok(auth),
        Some(auth) if auth.is_legacy() => Err(AppError::LegacyAuth),
        _ => Err(AppError::NoAuth),
    }
}
