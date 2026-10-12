//! Edge cases and the parts the first mutation audit found untested, each
//! with its expected value worked out by hand in the comments (C4).

use serde_json::Value;
use tool_core::api::analyze;

const STRINGS: &str = include_str!("../../../data/strings.json");
const SMALL: &str = include_str!("../../../tests/fixtures/small.csv");

/// Five rows. Row 5 of the file (header = row 1) has an unreadable date.
/// The first is completed "before" it was created; the second is completed
/// with no date. The latest date is 31 Mar 2026, the last day of 2026 Q1,
/// so nothing is in progress.
const EDGE: &str = "Created,Status,Resolved,Due
2026-01-10,Done,2026-01-05,
2026-01-20,Done,,
2026-02-01,Open,,2026-03-31
bad-date,Open,,
2026-03-31,Open,,2026-03-30
";

fn view(text: &str, settings: &str) -> Value {
    let out: Value = serde_json::from_str(&analyze(text, settings, STRINGS)).unwrap();
    assert!(out["error"].is_null(), "{out}");
    out["ok"].clone()
}

fn strs(v: &Value) -> Vec<String> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap().to_string())
        .collect()
}

fn insight_texts(v: &Value) -> Vec<String> {
    v["insights"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["text"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn edge_rows_are_counted_and_noted_in_the_singular() {
    let v = view(EDGE, "{}");
    assert_eq!(
        strs(&v["summary"]["notes"]),
        vec![
            "1 row was skipped because the \"Created\" date could not be read (the first is row 5: \"bad-date\").",
            "1 completed request has no completion date, so it counts as completed but not in any period's completed count.",
            "1 request has a completion date before its created date; it is counted as completed the day it was created.",
        ]
    );
    // 2026 Q1: four items, two open, two done: 50%. Completed in Q1: only
    // the first (its date clamps to 10 Jan); open at the end: the two
    // open; median of one completion taking 0 days is 0.
    let row = strs(&v["table"]["rows"][0]);
    assert_eq!(
        row,
        vec!["2026 Q1", "4", "2", "0", "2", "0", "\u{2014}", "\u{2014}", "50%", "1", "2", "0"]
    );
}

#[test]
fn due_on_the_as_of_day_is_not_overdue_and_age_must_exceed_the_limit() {
    // As of 31 Mar 2026. Open: 1 Feb (due 31 Mar, not past) and 31 Mar
    // (due 30 Mar, past). 1 Feb to 31 Mar is 58 days.
    let v = view(EDGE, r#"{"aging": "57"}"#);
    let t = insight_texts(&v);
    assert!(
        t.contains(&"1 open request is past the due date on 31 Mar 2026.".to_string()),
        "{t:?}"
    );
    assert!(
        t.contains(&"1 of 2 open requests (50%) has been open longer than 57 days.".to_string()),
        "{t:?}"
    );
    let v = view(EDGE, r#"{"aging": "58"}"#);
    assert!(!insight_texts(&v).iter().any(|i| i.contains("open longer")));
    assert_eq!(v["tiles"][3]["detail"], "1 past due, 0 open over 58 days");
}

#[test]
fn turning_a_role_off_turns_its_features_off() {
    // No due column: no overdue insight, and the open tile says nothing
    // about due dates.
    let v = view(EDGE, r#"{"due": "none"}"#);
    assert!(!insight_texts(&v)
        .iter()
        .any(|i| i.contains("past the due date")));
    assert_eq!(v["tiles"][3]["detail"], "0 open over 90 days");
    // No completion column: two charts, three tiles, and a note.
    let v = view(EDGE, r#"{"resolved": "none"}"#);
    let keys: Vec<&str> = v["charts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["key"].as_str().unwrap())
        .collect();
    assert_eq!(keys, vec!["received", "rate"]);
    assert_eq!(v["tiles"].as_array().unwrap().len(), 3);
    assert!(strs(&v["summary"]["notes"])
        .iter()
        .any(|n| n.starts_with("No completion-date column is chosen")));
    // A column number past the end falls back to the guess.
    let v = view(EDGE, r#"{"created": "99"}"#);
    assert_eq!(v["roles"]["created"], 0);
}

#[test]
fn the_year_over_year_chart_by_hand() {
    // small.csv by quarter: complete quarters 2025 Q1 to 2026 Q1, so two
    // years. 2025 is the oldest year, slot 1; 2026 slot 2. Q1: 3 in 2025,
    // 4 in 2026. Q2: 3 in 2025; 2026 Q2 is in progress, so not drawn.
    let v = view(SMALL, r#"{"granularity": "quarter"}"#);
    let years = v["charts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["key"] == "years")
        .unwrap()
        .clone();
    let legend: Vec<(String, String)> = years["legend"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| {
            (
                l["class"].as_str().unwrap().into(),
                l["label"].as_str().unwrap().into(),
            )
        })
        .collect();
    assert_eq!(
        legend,
        vec![
            ("s-y1".into(), "2025".into()),
            ("s-y2".into(), "2026".into())
        ]
    );
    assert_eq!(strs(&years["tips"][0]), vec!["Q1", "2025: 3", "2026: 4"]);
    assert_eq!(
        strs(&years["tips"][1]),
        vec!["Q2", "2025: 3", "2026: \u{2014}"]
    );
    // A year view has none.
    let v = view(SMALL, r#"{"granularity": "year"}"#);
    assert!(!v["charts"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c["key"] == "years"));
}

#[test]
fn a_group_of_five_or_more_below_target_is_flagged() {
    // Team A: six open items; team B: one done. Latest date 7 Jan 2026, so
    // 2026 Q1 is in progress and is the only period; its group rates cover
    // 1 Jan to 7 Jan. A: 0 of 6 = 0%, flagged. B: 1 of 1, fewer than five.
    let text = "Created,Status,Team\n2026-01-01,Open,A\n2026-01-02,Open,A\n2026-01-03,Open,A\n2026-01-04,Open,A\n2026-01-05,Open,A\n2026-01-06,Open,A\n2026-01-07,Done,B\n";
    let v = view(text, r#"{"granularity": "quarter"}"#);
    let rows: Vec<Vec<String>> = v["group_table"]["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(strs)
        .collect();
    assert_eq!(rows[0], vec!["A", "6", "6", "0% (below target)"]);
    assert_eq!(rows[1], vec!["B", "1", "0", "100%"]);
    assert!(insight_texts(&v).contains(
        &"Teams below the 80% target over the four quarters to 2026 Q1: A (0%).".to_string()
    ));
}

#[test]
fn the_completion_tile_shows_the_change_in_points() {
    // small.csv by month: April 2026 is in progress, so March is the last
    // complete month. March: T-9 pending, T-10 requested: 0%. February:
    // T-8 done: 100%. 0 - 100 = -100 points, which is bad.
    let v = view(SMALL, r#"{"granularity": "month"}"#);
    assert_eq!(v["tiles"][1]["label"], "Completion rate, Mar 2026");
    assert_eq!(v["tiles"][1]["value"], "0%");
    assert_eq!(
        v["tiles"][1]["deltas"][0]["text"],
        "\u{2212}100 pts on Feb 2026"
    );
    assert_eq!(v["tiles"][1]["deltas"][0]["tone"], "bad");
}

#[test]
fn past_sixteen_columns_only_the_last_four_carry_a_change_label() {
    // small.csv by month to 30 Jun 2026: Jan 2025 to Jun 2026 is eighteen
    // months. The last four: Mar 2026 (2 on 1: +100%), Apr (2 on 2: 0%),
    // May (0 on 2: -100%), Jun (0 on 0: none).
    let v = view(SMALL, r#"{"granularity": "month", "as_of": "2026-06-30"}"#);
    let received = v["charts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["key"] == "received")
        .unwrap()
        .clone();
    let labels: Vec<String> = received["marks"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|m| m["class"] == "bar-label")
        .map(|m| m["text"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(labels, vec!["+100%", "0%", "\u{2212}100%"]);
    // With sixteen months (to April 2026), every column gets its label.
    // Received by month: Jan 2025 1, Feb 1, Mar 1, Apr 3, May to Dec 0,
    // Jan 2026 1, Feb 1, Mar 2, Apr 2 (in progress). A change needs a
    // month before with more than 0: Feb, Mar, Apr, May 2025, Feb and Mar
    // 2026 = 6, plus "so far" over April 2026 = 7.
    let v = view(SMALL, r#"{"granularity": "month"}"#);
    let received = v["charts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["key"] == "received")
        .unwrap()
        .clone();
    let count = received["marks"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|m| m["class"] == "bar-label")
        .count();
    assert_eq!(count, 7);
}
