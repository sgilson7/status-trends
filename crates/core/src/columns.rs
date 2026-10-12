//! Guessing which column plays which part, from the header names. Every
//! guess can be overridden by the person; these are only the defaults.

use serde::Serialize;

use crate::csv::Table;
use crate::date::{self, NumericOrder};

/// The column chosen for each role, by index into the header.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Roles {
    pub created: Option<usize>,
    pub status: Option<usize>,
    pub resolved: Option<usize>,
    pub due: Option<usize>,
    pub group: Option<usize>,
}

fn norm(h: &str) -> String {
    h.trim().to_ascii_lowercase()
}

/// How strongly a header name suggests a role. 0 means not at all.
fn created_score(h: &str) -> u32 {
    let h = norm(h);
    match h.as_str() {
        "created" => 100,
        "created date" | "date created" | "created on" | "creation date" => 95,
        "opened" | "date opened" | "opened on" => 85,
        "assigned" | "assigned date" | "date assigned" | "assigned on" | "date set" | "set" => 80,
        "start date" | "start" => 50,
        "date" => 40,
        _ if h.contains("created") => 70,
        _ => 0,
    }
}

fn status_score(h: &str) -> u32 {
    let h = norm(h);
    match h.as_str() {
        "status" => 100,
        "state" | "issue status" => 90,
        "status category" => 40,
        _ if h.contains("status") => 70,
        "progress" => 50,
        _ => 0,
    }
}

fn resolved_score(h: &str) -> u32 {
    let h = norm(h);
    match h.as_str() {
        "resolved" => 100,
        "resolution date" | "resolved date" | "date resolved" | "resolved on" => 95,
        "completed" | "completed date" | "date completed" | "completed on" | "completion date" => 90,
        "done date" | "date done" | "turned in" | "turned in on" | "date turned in" => 85,
        "closed" | "closed date" | "date closed" | "closed on" => 80,
        "finished" | "finished on" => 60,
        _ => 0,
    }
}

fn due_score(h: &str) -> u32 {
    let h = norm(h);
    match h.as_str() {
        "due date" | "due" | "date due" | "due on" => 100,
        _ if h.contains("due") && !h.contains("overdue") => 60,
        _ => 0,
    }
}

fn group_score(h: &str) -> u32 {
    let h = norm(h);
    match h.as_str() {
        "team" | "student" | "student name" => 90,
        "component/s" | "components" | "component" | "section" | "class" => 80,
        "issue type" | "type" | "request type" | "unit" | "course" => 70,
        "project" | "project name" | "category" => 60,
        _ => 0,
    }
}

/// The best unused column for a role, if any column scores above zero.
fn best(header: &[String], score: fn(&str) -> u32, used: &[usize]) -> Option<usize> {
    let mut best: Option<(usize, u32)> = None;
    for (i, h) in header.iter().enumerate() {
        let s = score(h);
        if s > 0 && !used.contains(&i) && best.map(|(_, b)| s > b).unwrap_or(true) {
            best = Some((i, s));
        }
    }
    best.map(|(i, _)| i)
}

/// The first column where at least four in five of the first fifty
/// non-empty values read as dates. Used only when no header names a
/// created date.
fn first_date_column(table: &Table, used: &[usize]) -> Option<usize> {
    (0..table.header.len()).find(|&c| {
        if used.contains(&c) {
            return false;
        }
        let values: Vec<&str> = (0..table.rows.len())
            .map(|r| table.cell(r, c))
            .filter(|v| !v.trim().is_empty())
            .take(50)
            .collect();
        let dates = values
            .iter()
            .filter(|v| date::parse(v, NumericOrder::MonthFirst).is_some())
            .count();
        !values.is_empty() && dates * 5 >= values.len() * 4
    })
}

/// Guesses every role. Status is chosen first so a "Status" column is never
/// taken as a date; created, resolved and due next; the group last.
pub fn guess(table: &Table) -> Roles {
    let mut used = Vec::new();
    let status = best(&table.header, status_score, &used);
    used.extend(status);
    let created = best(&table.header, created_score, &used).or_else(|| first_date_column(table, &used));
    used.extend(created);
    let resolved = best(&table.header, resolved_score, &used);
    used.extend(resolved);
    let due = best(&table.header, due_score, &used);
    used.extend(due);
    let group = best(&table.header, group_score, &used);
    Roles {
        created,
        status,
        resolved,
        due,
        group,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::csv::parse;

    #[test]
    fn a_jira_export_header_is_read() {
        let t = parse("Summary,Issue key,Issue Type,Status,Priority,Resolution,Assignee,Created,Updated,Resolved,Due date,Status Category,Team\nx,T-1,Task,Done,High,Done,A,1/Jan/26,1/Jan/26,2/Jan/26,,Done,Web").unwrap();
        let r = guess(&t);
        assert_eq!(r.status, Some(3));
        assert_eq!(r.created, Some(7));
        assert_eq!(r.resolved, Some(9));
        assert_eq!(r.due, Some(10));
        assert_eq!(r.group, Some(12));
    }

    #[test]
    fn a_class_export_header_is_read() {
        let t = parse("Student,Unit,Assignment,Assigned,Due,Status,Turned in\nS1,U1,A,2025-09-01,2025-09-08,Graded,2025-09-07").unwrap();
        let r = guess(&t);
        assert_eq!(r.created, Some(3));
        assert_eq!(r.due, Some(4));
        assert_eq!(r.status, Some(5));
        assert_eq!(r.resolved, Some(6));
        assert_eq!(r.group, Some(0));
    }

    #[test]
    fn assignee_is_not_taken_for_a_date_and_resolution_not_for_resolved() {
        let t = parse("Assignee,Resolution,Status\nA,Done,Done").unwrap();
        let r = guess(&t);
        assert_eq!(r.created, None);
        assert_eq!(r.resolved, None);
        assert_eq!(r.status, Some(2));
    }

    #[test]
    fn without_a_named_date_the_first_column_of_dates_is_used() {
        let t = parse("Name,When,State\nA,2026-01-02,Open\nB,2026-01-03,Done\nC,,Open").unwrap();
        let r = guess(&t);
        assert_eq!(r.created, Some(1));
        assert_eq!(r.status, Some(2));
        let t = parse("Name,When\nA,soon\nB,later").unwrap();
        assert_eq!(guess(&t).created, None);
    }

    #[test]
    fn a_column_is_used_for_one_role_only() {
        let t = parse("Status,Status\nOpen,Open").unwrap();
        let r = guess(&t);
        assert_eq!(r.status, Some(0));
        assert_eq!(r.created, None);
    }
}
