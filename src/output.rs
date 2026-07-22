//! Colorized output.
//!
//! Ports the recursive `o()` printer and `format_relative_timestamp()` from
//! `helpers.php`. Handlers read the API into typed structs, then build a
//! `serde_json::Value` shaped like the arrays the PHP code printed and hand it
//! to [`print_value`]. This keeps one rendering rule instead of a bespoke
//! `Display` per endpoint. `owo-colors` auto-disables ANSI when stdout is not a
//! TTY, an improvement over the PHP escapes that were always emitted.

use chrono::{DateTime, Utc};
use owo_colors::OwoColorize;
use serde_json::Value;

/// Print a JSON value the way the PHP `o()` helper did.
///
/// - Object entries print as `Key: value` — key upper-cased and cyan, value
///   yellow. Nested objects/arrays recurse.
/// - Array entries print their value in `color` (default white).
/// - Scalars print directly in `color`.
pub fn print_value(value: &Value) {
    print_inner(value, "white");
}

fn print_inner(value: &Value, color: &str) {
    match value {
        Value::Object(map) => {
            for (key, val) in map {
                if val.is_object() || val.is_array() {
                    print_inner(val, color);
                } else {
                    // `Key: ` in cyan, then the scalar in yellow.
                    print!("{}", format!("{}: ", ucfirst(key)).cyan());
                    println!("{}", scalar_to_string(val).yellow());
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                print_inner(item, color);
            }
        }
        scalar => print_colored(&scalar_to_string(scalar), color),
    }
}

/// Print a plain line in one of the PHP palette colors.
pub fn line(text: &str, color: &str) {
    print_colored(text, color);
}

/// Print with no trailing newline (ports `o($x, ..., '', '')`), used by the
/// pipeline-wait progress dots.
pub fn inline(text: &str, color: &str) {
    match color {
        "red" => print!("{}", text.red()),
        "green" => print!("{}", text.green()),
        "yellow" => print!("{}", text.yellow()),
        "blue" => print!("{}", text.blue()),
        "magenta" => print!("{}", text.magenta()),
        "cyan" => print!("{}", text.cyan()),
        "gray" => print!("{}", text.bright_black()),
        _ => print!("{text}"),
    }
}

fn print_colored(text: &str, color: &str) {
    match color {
        "red" => println!("{}", text.red()),
        "green" => println!("{}", text.green()),
        "yellow" => println!("{}", text.yellow()),
        "blue" => println!("{}", text.blue()),
        "magenta" => println!("{}", text.magenta()),
        "cyan" => println!("{}", text.cyan()),
        "gray" => println!("{}", text.bright_black()),
        _ => println!("{text}"),
    }
}

/// Render a JSON scalar as the PHP would have echoed it (no quotes on strings).
fn scalar_to_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// Upper-case the first character (ports PHP `ucfirst`).
fn ucfirst(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Format an ISO-8601 timestamp as relative time, matching
/// `format_relative_timestamp()`: future dates and anything older than 7 days
/// print as `Mon DD, YYYY`; otherwise `N days/hours/minutes ago`.
pub fn format_relative_timestamp(date_string: &str) -> String {
    let parsed = match DateTime::parse_from_rfc3339(date_string) {
        Ok(dt) => dt.with_timezone(&Utc),
        Err(_) => return date_string.to_string(),
    };
    let now = Utc::now();
    let abs = |dt: DateTime<Utc>| dt.format("%b %d, %Y").to_string();

    // Future timestamp → absolute date.
    if parsed > now {
        return abs(parsed);
    }

    let diff = now - parsed;
    let days = diff.num_days();
    if days > 7 {
        abs(parsed)
    } else if days > 0 {
        format!("{days} day{} ago", plural(days))
    } else {
        let hours = diff.num_hours();
        if hours > 0 {
            format!("{hours} hour{} ago", plural(hours))
        } else {
            let mins = diff.num_minutes();
            format!("{mins} minute{} ago", plural(mins))
        }
    }
}

fn plural(n: i64) -> &'static str {
    if n > 1 {
        "s"
    } else {
        ""
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ucfirst_capitalizes() {
        assert_eq!(ucfirst("name"), "Name");
        assert_eq!(ucfirst(""), "");
    }

    #[test]
    fn relative_timestamp_far_past_is_absolute() {
        // 2000-01-01 is well over 7 days ago → absolute format.
        assert_eq!(
            format_relative_timestamp("2000-01-01T00:00:00+00:00"),
            "Jan 01, 2000"
        );
    }

    #[test]
    fn relative_timestamp_bad_input_passthrough() {
        assert_eq!(format_relative_timestamp("not-a-date"), "not-a-date");
    }
}
