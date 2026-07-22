//! Command aliases (new coverage toward `gh alias` parity).
//!
//! Aliases are stored locally in the config file as a `name -> expansion` map
//! and expanded before argument parsing. An alias cannot shadow a built-in
//! command. The expansion supports `$1..$N` positional placeholders; any
//! leftover arguments the template doesn't consume are appended.

use crate::cli::{AliasArgs, AliasCmd, BUILTIN_COMMANDS as BUILTINS};
use crate::config;
use crate::error::Result;
use crate::output;

pub fn run(args: AliasArgs) -> Result<()> {
    match args.cmd.unwrap_or(AliasCmd::List) {
        AliasCmd::List => list(),
        AliasCmd::Set { name, expansion } => set(&name, &expansion),
        AliasCmd::Delete { name } => delete(&name),
    }
}

fn list() -> Result<()> {
    let config = config::load()?;
    if config.aliases.is_empty() {
        output::line("No aliases defined.", "yellow");
        return Ok(());
    }
    for (name, expansion) in &config.aliases {
        output::line(&format!("{name}: {expansion}"), "cyan");
    }
    Ok(())
}

fn set(name: &str, expansion: &str) -> Result<()> {
    if BUILTINS.contains(&name) {
        return Err(crate::error::AppError::Usage(format!(
            "\"{name}\" is a built-in command and cannot be aliased."
        )));
    }
    let mut config = config::load()?;
    config
        .aliases
        .insert(name.to_string(), expansion.to_string());
    config::save(&config)?;
    output::line(&format!("Alias \"{name}\" set."), "green");
    Ok(())
}

fn delete(name: &str) -> Result<()> {
    let mut config = config::load()?;
    if config.aliases.remove(name).is_some() {
        config::save(&config)?;
        output::line(&format!("Alias \"{name}\" deleted."), "green");
    } else {
        output::line(&format!("No alias named \"{name}\"."), "yellow");
    }
    Ok(())
}

/// Expand a user alias in the raw argv, if present.
///
/// `argv[0]` is the binary. If `argv[1]` names an alias that isn't a built-in,
/// the alias expansion is tokenized, `$1..$N` are replaced with the trailing
/// arguments, and any unused trailing arguments are appended. On any failure
/// (no alias, config unreadable) the original argv is returned unchanged so
/// normal parsing proceeds.
pub fn expand_argv(argv: Vec<String>) -> Vec<String> {
    let Some(first) = argv.get(1) else {
        return argv;
    };
    if BUILTINS.contains(&first.as_str()) || first.starts_with('-') {
        return argv;
    }

    let config = match config::load() {
        Ok(c) => c,
        Err(_) => return argv,
    };
    let Some(expansion) = config.aliases.get(first) else {
        return argv;
    };

    let rest: Vec<String> = argv[2..].to_vec();
    let tokens = tokenize(expansion);
    let (expanded, used) = substitute(&tokens, &rest);

    // Rebuild argv: binary + expanded tokens + any unconsumed trailing args.
    let mut out = vec![argv[0].clone()];
    out.extend(expanded);
    for (idx, arg) in rest.iter().enumerate() {
        if !used.contains(&idx) {
            out.push(arg.clone());
        }
    }
    out
}

/// Replace `$1..$N` placeholders with the corresponding positional args.
/// Returns the expanded tokens and the set of arg indices that were consumed.
fn substitute(tokens: &[String], rest: &[String]) -> (Vec<String>, Vec<usize>) {
    let mut out = Vec::new();
    let mut used = Vec::new();
    for token in tokens {
        if let Some(n) = token
            .strip_prefix('$')
            .and_then(|d| d.parse::<usize>().ok())
        {
            if n >= 1 && n <= rest.len() {
                out.push(rest[n - 1].clone());
                used.push(n - 1);
                continue;
            }
        }
        out.push(token.clone());
    }
    (out, used)
}

/// Minimal whitespace tokenizer honoring single and double quotes.
fn tokenize(s: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    let mut has = false;
    for c in s.chars() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => cur.push(c),
            None if c == '\'' || c == '"' => {
                quote = Some(c);
                has = true;
            }
            None if c.is_whitespace() => {
                if has {
                    tokens.push(std::mem::take(&mut cur));
                    has = false;
                }
            }
            None => {
                cur.push(c);
                has = true;
            }
        }
    }
    if has {
        tokens.push(cur);
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_handles_quotes() {
        assert_eq!(
            tokenize(r#"pr list --title "hi there""#),
            vec!["pr", "list", "--title", "hi there"]
        );
    }

    #[test]
    fn substitute_replaces_positionals() {
        let (out, used) = substitute(&["pr".into(), "view".into(), "$1".into()], &["42".into()]);
        assert_eq!(out, vec!["pr", "view", "42"]);
        assert_eq!(used, vec![0]);
    }

    #[test]
    fn substitute_leaves_missing_placeholder() {
        let (out, _used) = substitute(&["$1".into()], &[]);
        assert_eq!(out, vec!["$1"]);
    }
}
