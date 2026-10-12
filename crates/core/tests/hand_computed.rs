//! Expected values worked out by hand from `tests/fixtures/small.csv` (C4),
//! never copied from the tool's output. The working is in the comments.
//!
//! The file: twelve Jira-style requests, T-1 to T-12, with Jira's default
//! date format. The latest date is T-11's completion on 5 Apr 2026, so the
//! as-of day is 5 Apr 2026 and 2026 Q2 is in progress.

use serde_json::Value;
use tool_core::api::analyze;

const SMALL: &str = include_str!("../../../tests/fixtures/small.csv");
const STRINGS: &str = include_str!("../../../data/strings.json");

fn view(settings: &str) -> Value {
    let out: Value = serde_json::from_str(&analyze(SMALL, settings, STRINGS)).unwrap();
    assert!(out["error"].is_null(), "{out}");
    out["ok"].clone()
}

fn table(v: &Value) -> Vec<Vec<String>> {
    v["table"]["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r.as_array().unwrap().iter().map(|c| c.as_str().unwrap().to_string()).collect())
        .collect()
}

fn insight(v: &Value, level: &str, text: &str) -> bool {
    v["insights"]
        .as_array()
        .unwrap()
        .iter()
        .any(|i| i["level"] == level && i["text"] == text)
}

#[test]
fn columns_and_statuses_are_guessed() {
    let v = view("{}");
    // Issue key, Summary, Status, Created, Resolved, Due date, Team.
    assert_eq!(v["roles"]["status"], 2);
    assert_eq!(v["roles"]["created"], 3);
    assert_eq!(v["roles"]["resolved"], 4);
    assert_eq!(v["roles"]["due"], 5);
    assert_eq!(v["roles"]["group"], 6);
    // Done 6 (T-1, 2, 5, 7, 8, 11), To Do 3, then one each, ties by name.
    let s: Vec<(String, i64, String)> = v["statuses"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| (s["name"].as_str().unwrap().into(), s["count"].as_i64().unwrap(), s["group"].as_str().unwrap().into()))
        .collect();
    assert_eq!(
        s,
        vec![
            ("Done".into(), 6, "done".into()),
            ("To Do".into(), 3, "requested".into()),
            ("In Progress".into(), 1, "pending".into()),
            ("In Review".into(), 1, "pending".into()),
            ("Won't Do".into(), 1, "excluded".into()),
        ]
    );
    assert_eq!(v["summary"]["text"], "12 requests from 3 Jan 2025 to 5 Apr 2026, read from 12 rows.");
}

#[test]
fn quarters_by_hand() {
    let v = view(r#"{"granularity": "quarter"}"#);
    // Columns: period, received, requested, pending, done, excluded,
    // change on previous, change on year before, rate, completed, open at
    // end, median days.
    //
    // 2025 Q1: T-1 done, T-2 done, T-3 pending. Completed in Q1: T-1 (Jan
    //   10), T-2 (Mar 15). Rate 2/3 = 66.7 -> 67%. Open at end: T-3 = 1.
    //   Days: T-1 3->10 Jan = 7; T-2 15 Feb -> 15 Mar = 28 (Feb 2025 has
    //   28 days). Median of 7 and 28 = 17.5, rounded down: 17.
    // 2025 Q2: T-4 requested, T-5 done, T-6 excluded. Rate 1/(3-1) = 50%.
    //   Change 3 on 3 = 0%. Completed: T-5 only (T-6 is excluded). Open at
    //   end: T-3, T-4 = 2. Days: T-5 10 -> 20 Apr = 10.
    // 2025 Q3, Q4: nothing received; open stays 2. Change on Q2 is -100%.
    // 2026 Q1: T-7 done, T-8 done, T-9 pending, T-10 requested. Rate 2/4 =
    //   50%. Change on 2025 Q4 (0): none. On 2025 Q1 (3): 1/3 -> +33%.
    //   Completed: T-7 (Jan 25), T-8 (Feb 16). Open at end: T-3, T-4, T-9,
    //   T-10 = 4. Days: 20 and 10, median 15.
    // 2026 Q2 (in progress): T-11 done, T-12 requested. Rate 1/2 = 50%.
    //   Change on Q1: 2 on 4 = -50%. On 2025 Q2 (3): -1/3 -> -33%.
    //   Completed: T-11. Open on 5 Apr: 4 + T-12 = 5. Days: 1 -> 5 Apr = 4.
    let m = "\u{2212}";
    let d = "\u{2014}";
    let expected: Vec<Vec<String>> = vec![
        vec!["2025 Q1", "3", "0", "1", "2", "0", d, d, "67%", "2", "1", "17"],
        vec!["2025 Q2", "3", "1", "0", "1", "1", "0%", d, "50%", "1", "2", "10"],
        vec!["2025 Q3", "0", "0", "0", "0", "0", &format!("{m}100%"), d, d, "0", "2", d],
        vec!["2025 Q4", "0", "0", "0", "0", "0", d, d, d, "0", "2", d],
        vec!["2026 Q1", "4", "1", "1", "2", "0", d, "+33%", "50%", "2", "4", "15"],
        vec!["2026 Q2 (in progress)", "2", "1", "0", "1", "0", &format!("{m}50%"), &format!("{m}33%"), "50%", "1", "5", "4"],
    ]
    .into_iter()
    .map(|r| r.into_iter().map(String::from).collect())
    .collect();
    assert_eq!(table(&v), expected);
}

#[test]
fn quarter_tiles_and_insights_by_hand() {
    let v = view(r#"{"granularity": "quarter"}"#);
    // The last complete quarter is 2026 Q1.
    let tiles = v["tiles"].as_array().unwrap();
    assert_eq!(tiles[0]["label"], "Requests received in 2026 Q1");
    assert_eq!(tiles[0]["value"], "4");
    assert_eq!(tiles[0]["deltas"][0]["text"], "+33% on 2025 Q1");
    assert_eq!(tiles[1]["value"], "50%");
    assert_eq!(tiles[1]["detail"], "Target 80%, below target");
    assert_eq!(tiles[2]["value"], "2");
    assert_eq!(tiles[2]["detail"], "Median 15 days to complete");
    // Open now: T-3, T-4, T-9, T-10, T-12 = 5, up from 4 at the end of Q1:
    // +25%. Past due on 5 Apr: T-3 (due 1 Apr 2025), T-9 (due 15 Mar 2026).
    // Open over 90 days: T-3 (from 20 Mar 2025), T-4 (from 2 Apr 2025).
    assert_eq!(tiles[3]["value"], "5");
    assert_eq!(tiles[3]["detail"], "2 past due, 2 open over 90 days");
    assert_eq!(tiles[3]["deltas"][0]["text"], "+25% since the end of 2026 Q1");
    assert_eq!(tiles[3]["deltas"][0]["tone"], "bad");

    assert!(insight(&v, "note", "2026 Q2 is still in progress (data to 5 Apr 2026), so comparisons use 2026 Q1."));
    assert!(insight(&v, "risk", "50% of the requests received in 2026 Q1 are completed, below the 80% target (2 of 4)."));
    // Open at end: 2 at the end of 2025 Q4, 4 at the end of 2026 Q1: +100%.
    assert!(insight(&v, "watch", "Open requests rose 100% in 2026 Q1: 2 at the end of 2025 Q4, 4 at the end of 2026 Q1."));
    // 2 of 5 open = 40%, at least 25%: a risk.
    assert!(insight(&v, "risk", "2 of 5 open requests (40%) have been open longer than 90 days."));
    assert!(insight(&v, "risk", "2 open requests are past the due date on 5 Apr 2026."));
    assert!(insight(&v, "watch", "Requests received rose 33% on 2025 Q1: 3 in 2025 Q1, 4 in 2026 Q1."));
    // Risks come first.
    assert_eq!(v["insights"][0]["level"], "risk");
}

#[test]
fn years_by_hand() {
    let v = view(r#"{"granularity": "year"}"#);
    // 2025: six received (T-1 to T-6), done T-1, T-2, T-5 = 3 of 5 counted
    // = 60%. 2026: six received, in progress.
    let t = table(&v);
    assert_eq!(t[0][0], "2025");
    assert_eq!(t[0][1], "6");
    assert_eq!(t[0][7], "60%");
    assert_eq!(t[1][0], "2026 (in progress)");
    assert_eq!(t[1][6], "0%");
    // A year view has no "change on year before" column.
    assert_eq!(t[0].len(), 11);
}

#[test]
fn an_october_fiscal_year_by_hand() {
    let v = view(r#"{"granularity": "year", "fiscal_start": "10"}"#);
    // FY25 = Oct 2024 to Sep 2025: T-1 to T-6. FY26 = Oct 2025 to Sep 2026:
    // T-7 to T-12, in progress.
    let t = table(&v);
    assert_eq!(t.len(), 2);
    assert_eq!((t[0][0].as_str(), t[0][1].as_str()), ("FY25", "6"));
    assert_eq!((t[1][0].as_str(), t[1][1].as_str()), ("FY26 (in progress)", "6"));
}

#[test]
fn the_group_table_and_the_group_filter_by_hand() {
    let v = view(r#"{"granularity": "quarter"}"#);
    // Data: T-3, T-4, T-6, T-8, T-9, T-12 (6). Web: T-1, T-2, T-5, T-7,
    // T-10, T-11 (6). Tie on count: by name, Data first.
    let g = &v["group_table"];
    assert_eq!(g["headers"][0], "Team");
    let rows = g["rows"].as_array().unwrap();
    // Data in 2026 Q1: T-8, T-9 = 2; in 2025 Q4: 0, so no change. Open now:
    // T-3, T-4, T-9, T-12 = 4. Past due: T-3, T-9 = 2. Complete periods
    // (created before 1 Apr 2026), excluded aside: T-3, T-4, T-8, T-9; done
    // T-8: 1 of 4 = 25%. Fewer than five counted, so not flagged.
    assert_eq!(rows[0], serde_json::json!(["Data", "2", "\u{2014}", "4", "2", "25%"]));
    // Web: 2026 Q1: T-7, T-10 = 2. Open: T-10 = 1. Past due: 0. Complete
    // periods: T-1, T-2, T-5, T-7, T-10: 4 of 5 done = 80%.
    assert_eq!(rows[1], serde_json::json!(["Web", "2", "\u{2014}", "1", "0", "80%"]));

    let v = view(r#"{"granularity": "quarter", "only_group": "Web"}"#);
    // Web only: 2026 Q1 received 2 (T-7 done, T-10 requested).
    let t = table(&v);
    let q1 = t.iter().find(|r| r[0] == "2026 Q1").unwrap();
    assert_eq!(&q1[1..6], &["2", "1", "0", "1", "0"]);
    assert!(v["group_table"].is_null());
    assert!(v["summary"]["notes"][0].as_str().unwrap().starts_with("Showing only \"Web\": 6 other rows"));
}

#[test]
fn moving_a_status_changes_the_counts() {
    // Moving "In Review" (T-9) to done raises 2026 Q1 to 3 done of 4 = 75%.
    let v = view(r#"{"granularity": "quarter", "statuses": {"In Review": "done"}}"#);
    let t = table(&v);
    let q1 = t.iter().find(|r| r[0] == "2026 Q1").unwrap();
    assert_eq!(q1[4], "3");
    assert_eq!(q1[8], "75%");
    let moved = v["statuses"].as_array().unwrap().iter().find(|s| s["name"] == "In Review").unwrap().clone();
    assert_eq!(moved["moved"], true);
}

#[test]
fn the_csv_export_matches_the_table() {
    let v = view(r#"{"granularity": "quarter"}"#);
    let csv = v["export_csv"].as_str().unwrap();
    let lines: Vec<&str> = csv.split("\r\n").collect();
    assert_eq!(lines[0], "Period,Requests received,Still requested,Pending,Completed,Excluded,Change on period before,Change on year before,Completion rate,Completed in period,Open at end,Median days to complete");
    assert_eq!(lines[1], "2025 Q1,3,0,1,2,0,,,67%,2,1,17");
    assert_eq!(lines[6], "2026 Q2 (in progress),2,1,0,1,0,-50%,-33%,50%,1,5,4");
    assert_eq!(v["export_name"], "status-trends-by-quarter.csv");
}

#[test]
fn an_as_of_day_cuts_the_data() {
    // As of 31 Mar 2026: T-11 and T-12 are left out; 2026 Q1 ends on its
    // last day, so nothing is in progress.
    let v = view(r#"{"granularity": "quarter", "as_of": "2026-03-31"}"#);
    let t = table(&v);
    assert_eq!(t.last().unwrap()[0], "2026 Q1");
    assert!(v["summary"]["notes"].as_array().unwrap().iter().any(|n| n == "2 rows created after 31 Mar 2026 are left out."));
}
