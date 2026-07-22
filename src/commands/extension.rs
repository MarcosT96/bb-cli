//! Extension system (`gh extension` analogue).
//!
//! An extension is a repo named `bb-<name>` containing an executable of the
//! same name. Installed extensions live under `~/.bb/extensions/<name>/`.
//! `bb <name> ...` forwards all arguments to that executable verbatim (bb does
//! not interpret them). Extensions cannot shadow built-in commands.

use std::path::PathBuf;
use std::process::Command;

use crate::cli::{ExtensionArgs, ExtensionCmd, BUILTIN_COMMANDS as BUILTINS};
use crate::error::{AppError, Result};
use crate::output;

pub fn run(args: ExtensionArgs) -> Result<()> {
    match args.cmd.unwrap_or(ExtensionCmd::List) {
        ExtensionCmd::List => list(),
        ExtensionCmd::Install { source } => install(&source),
        ExtensionCmd::Remove { name } => remove(&name),
    }
}

/// The extensions root: `~/.bb/extensions`.
fn extensions_dir() -> Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| AppError::Repo("Could not resolve HOME directory.".into()))?;
    Ok(home.join(".bb").join("extensions"))
}

/// Path to an extension's executable: `~/.bb/extensions/<name>/bb-<name>`.
fn executable_path(name: &str) -> Result<PathBuf> {
    Ok(extensions_dir()?.join(name).join(format!("bb-{name}")))
}

fn list() -> Result<()> {
    let dir = extensions_dir()?;
    if !dir.exists() {
        output::line("No extensions installed.", "yellow");
        return Ok(());
    }
    let mut found = false;
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            if let Some(name) = entry.file_name().to_str() {
                if executable_path(name)?.exists() {
                    output::line(name, "cyan");
                    found = true;
                }
            }
        }
    }
    if !found {
        output::line("No extensions installed.", "yellow");
    }
    Ok(())
}

fn install(source: &str) -> Result<()> {
    // Accept a full git URL or "owner/repo" (assumed Bitbucket).
    let (url, repo_name) = if source.contains("://") || source.contains('@') {
        let name = source
            .trim_end_matches(".git")
            .rsplit(['/', ':'])
            .next()
            .unwrap_or_default()
            .to_string();
        (source.to_string(), name)
    } else {
        let name = source.rsplit('/').next().unwrap_or_default().to_string();
        (format!("https://bitbucket.org/{source}.git"), name)
    };

    // The extension name is the repo name minus the "bb-" prefix.
    let name = repo_name
        .strip_prefix("bb-")
        .ok_or_else(|| {
            AppError::Usage(format!(
                "Extension repos must be named bb-<name> (got \"{repo_name}\")."
            ))
        })?
        .to_string();

    if BUILTINS.contains(&name.as_str()) {
        return Err(AppError::Usage(format!(
            "\"{name}\" is a built-in command and cannot be an extension."
        )));
    }

    let dest = extensions_dir()?.join(&name);
    if dest.exists() {
        return Err(AppError::Usage(format!(
            "Extension \"{name}\" is already installed. Remove it first."
        )));
    }
    std::fs::create_dir_all(extensions_dir()?)?;

    output::line(&format!("Installing {name} from {url} ..."), "green");
    let status = Command::new("git")
        .arg("clone")
        .arg(&url)
        .arg(&dest)
        .status()?;
    if !status.success() {
        return Err(AppError::Repo("git clone failed.".into()));
    }

    if !executable_path(&name)?.exists() {
        return Err(AppError::Usage(format!(
            "Cloned, but no executable named bb-{name} was found in the repo."
        )));
    }
    output::line(&format!("Installed extension \"{name}\"."), "green");
    Ok(())
}

fn remove(name: &str) -> Result<()> {
    let dir = extensions_dir()?.join(name);
    if !dir.exists() {
        output::line(&format!("No extension named \"{name}\"."), "yellow");
        return Ok(());
    }
    std::fs::remove_dir_all(&dir)?;
    output::line(&format!("Removed extension \"{name}\"."), "green");
    Ok(())
}

/// If `argv[1]` names an installed extension (and not a built-in), exec it with
/// the remaining arguments and never return. Any failure to resolve simply
/// returns so normal command parsing proceeds.
pub fn maybe_dispatch(argv: &[String]) {
    let Some(name) = argv.get(1) else { return };
    if BUILTINS.contains(&name.as_str()) || name.starts_with('-') {
        return;
    }
    let Ok(exe) = executable_path(name) else {
        return;
    };
    if !exe.exists() {
        return;
    }

    let status = Command::new(&exe).args(&argv[2..]).status();
    match status {
        Ok(s) => std::process::exit(s.code().unwrap_or(1)),
        Err(e) => {
            eprintln!("Failed to run extension \"{name}\": {e}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn executable_path_shape() {
        let p = executable_path("hello").unwrap();
        assert!(p.ends_with("hello/bb-hello"));
    }
}
