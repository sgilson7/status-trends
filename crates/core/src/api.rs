//! The one door into the core. Every function takes strings and returns a JSON
//! string: `{"ok": ...}` on success or `{"error": "..."}` with a message the
//! page can show as it is. The shim forwards these calls and nothing else.

use serde::Serialize;

use crate::csv;
use crate::dashboard;
use crate::settings::{Raw, Settings};
use crate::strings;

fn ok<T: Serialize>(value: &T) -> String {
    serde_json::json!({ "ok": value }).to_string()
}

fn error(message: String) -> String {
    serde_json::json!({ "error": message }).to_string()
}

/// Checks the strings file and returns the strings the page should draw.
pub fn load(strings_json: &str) -> String {
    match strings::parse(strings_json) {
        Ok(s) => ok(&s),
        Err(e) => error(e.message()),
    }
}

/// Reads a delimited text file with the person's settings and returns the
/// whole dashboard view.
pub fn analyze(text: &str, settings_json: &str, strings_json: &str) -> String {
    let strings = match strings::parse(strings_json) {
        Ok(s) => s,
        Err(e) => return error(e.message()),
    };
    let raw: Raw = match serde_json::from_str(settings_json) {
        Ok(r) => r,
        Err(e) => return error(format!("The settings could not be read: {e}")),
    };
    let table = match csv::parse(text) {
        Ok(t) => t,
        Err(e) => return error(e.message()),
    };
    let mut settings = Settings::read(&raw);
    ok(&dashboard::build(&table, &mut settings, &strings))
}

#[cfg(test)]
mod tests {
    use super::*;

    const STRINGS: &str = include_str!("../../../data/strings.json");

    #[test]
    fn load_wraps_success_and_failure() {
        assert!(load(STRINGS).starts_with("{\"ok\":"));
        assert_eq!(
            load(r#"{"ui": {"title": ""}, "vocabularies": []}"#),
            r#"{"error":"The strings file has an empty label \"title\"."}"#
        );
    }

    #[test]
    fn analyze_reports_each_kind_of_failure_as_an_error() {
        assert_eq!(analyze("", "{}", STRINGS), r#"{"error":"The file is empty."}"#);
        assert!(analyze("a,b\n1,2", "{", STRINGS).starts_with(r#"{"error":"The settings could not be read"#));
        assert!(analyze("a,b\n1,2", "{}", "{}").starts_with(r#"{"error":"The strings file could not be read"#));
    }

    #[test]
    fn a_file_without_dates_returns_a_problem_and_the_columns() {
        let v: serde_json::Value = serde_json::from_str(&analyze("Name,Status\nA,Open", "{}", STRINGS)).unwrap();
        assert_eq!(v["ok"]["columns"][1]["name"], "Status");
        assert!(v["ok"]["problem"].as_str().unwrap().starts_with("No column looks like the date"));
        assert!(v["ok"]["charts"].as_array().unwrap().is_empty());
    }

    #[test]
    fn no_input_makes_any_function_panic() {
        let odd = [
            "", "\u{feff}", "\"", "a\n\"", ",,,\n,,,", "Created\n2026-01-01", "Created,Status\n9999-99-99,Done",
            "Created,Resolved,Status\n2026-01-01,2025-01-01,Done", "Created\n1/1/1900\n12/31/2200",
        ];
        let settings = [
            "{}", r#"{"granularity": "month", "show": "1"}"#, r#"{"created": "99", "status": "-1", "show": "0"}"#,
            r#"{"granularity": "year", "fiscal_start": "12", "as_of": "1900-01-01"}"#,
            r#"{"only_group": "nobody", "group": "0"}"#, r#"{"as_of": "2200-12-31", "granularity": "month"}"#,
        ];
        for text in odd {
            for s in settings {
                let out = analyze(text, s, STRINGS);
                assert!(out.starts_with("{\"ok\"") || out.starts_with("{\"error\""), "{text:?} {s}: {out}");
            }
        }
    }
}
