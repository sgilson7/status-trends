//! The person's choices, as the page sends them: every value is the text of
//! a control. The core reads and checks them here, so the page never parses
//! a number. A value that cannot be used falls back to its default and
//! leaves a note saying so.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::date::{self, Day, NumericOrder};
use crate::period::{Calendar, Granularity};
use crate::status::Group;

/// The page's controls, all as text. Missing fields take their defaults.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Raw {
    pub granularity: String,
    pub fiscal_start: String,
    pub show: String,
    pub vocabulary: String,
    /// Each role: "" to guess, "none" for no column, or a column number.
    pub created: String,
    pub status: String,
    pub resolved: String,
    pub due: String,
    pub group: String,
    /// "auto", "month" or "day": how to read a date like 10/12/2026.
    pub date_order: String,
    pub as_of: String,
    pub target: String,
    pub growth: String,
    pub aging: String,
    /// Only items whose group column holds this value; "" for all.
    pub only_group: String,
    /// Status name to group key, for the statuses the person moved.
    pub statuses: BTreeMap<String, String>,
}

/// A role as chosen in the page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    Guess,
    None,
    Column(usize),
}

fn choice(text: &str) -> Choice {
    match text.trim() {
        "" => Choice::Guess,
        "none" => Choice::None,
        t => t.parse().map(Choice::Column).unwrap_or(Choice::Guess),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    pub calendar: Calendar,
    /// How many periods the charts and table show; `None` for all.
    pub show: Option<usize>,
    pub vocabulary: String,
    pub created: Choice,
    pub status: Choice,
    pub resolved: Choice,
    pub due: Choice,
    pub group: Choice,
    /// `None` to detect from the file.
    pub date_order: Option<NumericOrder>,
    pub as_of_text: String,
    /// Percent of items completed that counts as on target.
    pub target: i64,
    /// Percent change that counts as a jump.
    pub growth: i64,
    /// Days open that counts as too long.
    pub aging: i64,
    pub only_group: String,
    pub statuses: BTreeMap<String, Group>,
    /// Notes about values that could not be used.
    pub notes: Vec<String>,
}

pub const DEFAULT_TARGET: i64 = 80;
pub const DEFAULT_GROWTH: i64 = 20;
pub const DEFAULT_AGING: i64 = 90;

/// A whole number in a range, or the default with a note naming the field.
fn whole(
    text: &str,
    name: &str,
    low: i64,
    high: i64,
    default: i64,
    notes: &mut Vec<String>,
) -> i64 {
    let t = text.trim().trim_end_matches('%').trim();
    if t.is_empty() {
        return default;
    }
    match t.parse::<i64>() {
        Ok(v) if (low..=high).contains(&v) => v,
        _ => {
            notes.push(format!(
                "The {name} must be a whole number from {low} to {high}, not \"{}\"; using {default}.",
                text.trim()
            ));
            default
        }
    }
}

impl Settings {
    pub fn read(raw: &Raw) -> Settings {
        let mut notes = Vec::new();
        let granularity =
            Granularity::from_key(raw.granularity.trim()).unwrap_or(Granularity::Quarter);
        let fiscal_start = whole(
            &raw.fiscal_start,
            "fiscal year start month",
            1,
            12,
            1,
            &mut notes,
        );
        let show = match raw.show.trim() {
            "" | "all" => None,
            t => t.parse::<usize>().ok().filter(|n| *n > 0),
        };
        let date_order = match raw.date_order.trim() {
            "month" => Some(NumericOrder::MonthFirst),
            "day" => Some(NumericOrder::DayFirst),
            _ => None,
        };
        let target = whole(
            &raw.target,
            "completion target",
            1,
            100,
            DEFAULT_TARGET,
            &mut notes,
        );
        let growth = whole(
            &raw.growth,
            "change alert",
            1,
            1000,
            DEFAULT_GROWTH,
            &mut notes,
        );
        let aging = whole(
            &raw.aging,
            "open-too-long limit",
            1,
            3650,
            DEFAULT_AGING,
            &mut notes,
        );
        let statuses = raw
            .statuses
            .iter()
            .filter_map(|(name, key)| Group::from_key(key).map(|g| (name.clone(), g)))
            .collect();
        Settings {
            calendar: Calendar {
                granularity,
                fiscal_start,
            },
            show,
            vocabulary: raw.vocabulary.trim().to_string(),
            created: choice(&raw.created),
            status: choice(&raw.status),
            resolved: choice(&raw.resolved),
            due: choice(&raw.due),
            group: choice(&raw.group),
            date_order,
            as_of_text: raw.as_of.trim().to_string(),
            target,
            growth,
            aging,
            only_group: raw.only_group.clone(),
            statuses,
            notes,
        }
    }

    /// The typed "as of" day, read in the file's date order. A typed day
    /// that cannot be read leaves a note and is ignored.
    pub fn as_of(&mut self, order: NumericOrder) -> Option<Day> {
        if self.as_of_text.is_empty() {
            return None;
        }
        let day = date::parse(&self.as_of_text, order);
        if day.is_none() {
            self.notes.push(format!(
                "The \"as of\" date \"{}\" could not be read, so the latest date in the file is used.",
                self.as_of_text
            ));
        }
        day
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(json: &str) -> Settings {
        Settings::read(&serde_json::from_str(json).unwrap())
    }

    #[test]
    fn empty_settings_take_the_defaults() {
        let s = read("{}");
        assert_eq!(s.calendar.granularity, Granularity::Quarter);
        assert_eq!(s.calendar.fiscal_start, 1);
        assert_eq!(s.show, None);
        assert_eq!((s.target, s.growth, s.aging), (80, 20, 90));
        assert_eq!(s.created, Choice::Guess);
        assert_eq!(s.date_order, None);
        assert!(s.notes.is_empty());
    }

    #[test]
    fn controls_are_read() {
        let s = read(
            r#"{"granularity": "month", "fiscal_start": "10", "show": "12", "created": "3", "due": "none", "date_order": "day", "target": "75%", "growth": " 10 ", "aging": "30", "statuses": {"Blocked": "requested", "Odd": "nonsense"}}"#,
        );
        assert_eq!(s.calendar.granularity, Granularity::Month);
        assert_eq!(s.calendar.fiscal_start, 10);
        assert_eq!(s.show, Some(12));
        assert_eq!(s.created, Choice::Column(3));
        assert_eq!(s.due, Choice::None);
        assert_eq!(s.date_order, Some(NumericOrder::DayFirst));
        assert_eq!((s.target, s.growth, s.aging), (75, 10, 30));
        assert_eq!(s.statuses.get("Blocked"), Some(&Group::Requested));
        assert_eq!(s.statuses.get("Odd"), None);
    }

    #[test]
    fn unusable_numbers_fall_back_with_a_note() {
        let s = read(r#"{"target": "120", "growth": "lots", "aging": "-5", "fiscal_start": "13"}"#);
        assert_eq!((s.target, s.growth, s.aging), (80, 20, 90));
        assert_eq!(s.calendar.fiscal_start, 1);
        assert_eq!(s.notes.len(), 4);
        assert_eq!(
            s.notes[1],
            "The completion target must be a whole number from 1 to 100, not \"120\"; using 80."
        );
    }

    #[test]
    fn an_unreadable_as_of_day_is_noted_and_ignored() {
        let mut s = read(r#"{"as_of": "someday"}"#);
        assert_eq!(s.as_of(NumericOrder::MonthFirst), None);
        assert_eq!(s.notes.len(), 1);
        let mut s = read(r#"{"as_of": "2026-04-05"}"#);
        assert_eq!(
            s.as_of(NumericOrder::MonthFirst),
            Some(date::days_from_civil(2026, 4, 5))
        );
        let mut s = read("{}");
        assert_eq!(s.as_of(NumericOrder::MonthFirst), None);
        assert!(s.notes.is_empty());
    }
}
