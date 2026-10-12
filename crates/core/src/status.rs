//! The four status groups every status is sorted into, and the default
//! guess for a status name. The person can move any status to another group.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Group {
    /// Asked for and not started: To Do, Open, Backlog, Not started.
    Requested,
    /// Started and not finished: In Progress, In Review, Blocked.
    Pending,
    /// Finished: Done, Closed, Resolved, Graded.
    Done,
    /// Will not be done, and so not counted against completion: Won't Do,
    /// Duplicate, Cancelled, Excused.
    Excluded,
}

/// The order groups are listed and stacked in, from the baseline up.
pub const STACK_ORDER: [Group; 4] = [Group::Done, Group::Pending, Group::Requested, Group::Excluded];

impl Group {
    pub fn key(self) -> &'static str {
        match self {
            Group::Requested => "requested",
            Group::Pending => "pending",
            Group::Done => "done",
            Group::Excluded => "excluded",
        }
    }

    pub fn from_key(key: &str) -> Option<Group> {
        match key {
            "requested" => Some(Group::Requested),
            "pending" => Some(Group::Pending),
            "done" => Some(Group::Done),
            "excluded" => Some(Group::Excluded),
            _ => None,
        }
    }

    /// Still open: requested or pending.
    pub fn is_open(self) -> bool {
        matches!(self, Group::Requested | Group::Pending)
    }
}

const EXCLUDED_WORDS: &[&str] = &[
    "won't", "wont", "will not", "cancel", "duplicate", "reject", "declin", "invalid",
    "obsolete", "excused", "exempt", "withdrawn", "not applicable", "abandon",
];
const DONE_WORDS: &[&str] = &[
    "done", "closed", "resolved", "complete", "finished", "released", "deployed", "shipped",
    "delivered", "fixed", "graded", "turned in", "mastered", "passed", "approved", "accepted",
];
const REQUESTED_WORDS: &[&str] = &[
    "to do", "todo", "open", "new", "backlog", "not started", "requested", "submitted",
    "triage", "selected", "missing", "assigned", "queued",
];
const PENDING_WORDS: &[&str] = &[
    "progress", "review", "pending", "waiting", "awaiting", "blocked", "hold", "testing", "qa",
    "doing", "develop", "investigat", "verif", "validat", "started", "draft", "revis", "late",
];

/// The default group for a status name: excluded words first, then done,
/// then the requested words, then pending; anything else is requested.
pub fn guess(status: &str) -> Group {
    let s = status.trim().to_ascii_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| s.contains(w));
    if has(EXCLUDED_WORDS) {
        Group::Excluded
    } else if has(DONE_WORDS) && !s.starts_with("not ") && !s.contains("incomplete") {
        Group::Done
    } else if has(REQUESTED_WORDS) {
        Group::Requested
    } else if has(PENDING_WORDS) {
        Group::Pending
    } else {
        Group::Requested
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jira_statuses_are_sorted() {
        for (s, g) in [
            ("To Do", Group::Requested),
            ("Open", Group::Requested),
            ("Backlog", Group::Requested),
            ("Reopened", Group::Requested),
            ("Selected for Development", Group::Requested),
            ("In Progress", Group::Pending),
            ("In Review", Group::Pending),
            ("Blocked", Group::Pending),
            ("Waiting for customer", Group::Pending),
            ("Done", Group::Done),
            ("Closed", Group::Done),
            ("Resolved", Group::Done),
            ("Won't Do", Group::Excluded),
            ("Duplicate", Group::Excluded),
            ("Cancelled", Group::Excluded),
        ] {
            assert_eq!(guess(s), g, "{s}");
        }
    }

    #[test]
    fn class_statuses_are_sorted() {
        for (s, g) in [
            ("Not started", Group::Requested),
            ("Missing", Group::Requested),
            ("In progress", Group::Pending),
            ("Late", Group::Pending),
            ("Turned in", Group::Done),
            ("Graded", Group::Done),
            ("Excused", Group::Excluded),
            ("Not complete", Group::Requested),
            ("Incomplete", Group::Requested),
        ] {
            assert_eq!(guess(s), g, "{s}");
        }
    }

    #[test]
    fn an_unknown_status_counts_as_requested() {
        assert_eq!(guess("Zebra"), Group::Requested);
        assert_eq!(guess(""), Group::Requested);
    }

    #[test]
    fn keys_round_trip() {
        for g in STACK_ORDER {
            assert_eq!(Group::from_key(g.key()), Some(g));
        }
        assert_eq!(Group::from_key("other"), None);
        assert!(Group::Pending.is_open() && Group::Requested.is_open());
        assert!(!Group::Done.is_open() && !Group::Excluded.is_open());
    }
}
