//! From a table and the settings to the numbers: the items, the counts per
//! period, and the changes between periods. Every count the dashboard shows
//! is made here.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::columns::{self, Roles};
use crate::csv::Table;
use crate::date::{self, Day, NumericOrder};
use crate::settings::{Choice, Settings};
use crate::status::{self, Group};

/// One row of the file, read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub created: Day,
    pub group: Group,
    /// The completion day, kept only for completed items.
    pub resolved: Option<Day>,
    pub due: Option<Day>,
    /// The value in the group column, or "".
    pub team: String,
}

/// A status as found in the file, with the group it is counted in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StatusUse {
    pub name: String,
    pub count: usize,
    pub group: Group,
    /// True when the person moved it from the guessed group.
    pub moved: bool,
}

/// What the reading found, for the summary and the notes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Reading {
    pub rows: usize,
    pub used: usize,
    pub skipped_no_date: usize,
    /// The file row (header = row 1) and text of the first skipped date.
    pub first_skipped: Option<(usize, String)>,
    pub after_as_of: usize,
    pub done_without_date: usize,
    pub resolved_before_created: usize,
    pub other_group: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Data {
    pub roles: Roles,
    pub order: NumericOrder,
    pub statuses: Vec<StatusUse>,
    /// Every group value with its item count, most items first.
    pub teams: Vec<(String, usize)>,
    pub items: Vec<Item>,
    pub as_of: Day,
    pub has_resolved: bool,
    pub has_due: bool,
    pub reading: Reading,
}

fn pick(choice: Choice, guess: Option<usize>, width: usize) -> Option<usize> {
    match choice {
        Choice::Guess => guess,
        Choice::None => None,
        Choice::Column(c) if c < width => Some(c),
        Choice::Column(_) => guess,
    }
}

/// Why the file cannot be turned into a dashboard yet. The page still shows
/// the column choices, so the person can fix it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    NoCreatedColumn,
    NoDates { column: String },
}

impl Problem {
    pub fn message(&self) -> String {
        match self {
            Problem::NoCreatedColumn => "No column looks like the date each item was created. Choose it under \"Columns and statuses\".".to_string(),
            Problem::NoDates { column } => format!(
                "No date in the column \"{column}\" could be read. Choose the column that holds the date each item was created, or change how dates are read."
            ),
        }
    }
}

/// Reads the items. `Err` carries the roles found so far with the problem.
pub fn read(table: &Table, settings: &mut Settings) -> Result<Data, (Roles, NumericOrder, Problem)> {
    let width = table.header.len();
    let guess = columns::guess(table);
    let roles = Roles {
        created: pick(settings.created, guess.created, width),
        status: pick(settings.status, guess.status, width),
        resolved: pick(settings.resolved, guess.resolved, width),
        due: pick(settings.due, guess.due, width),
        group: pick(settings.group, guess.group, width),
    };
    let column = |c: Option<usize>| move |r: usize| c.map(|c| table.cell(r, c)).unwrap_or("");
    let created_of = column(roles.created);
    let status_of = column(roles.status);
    let resolved_of = column(roles.resolved);
    let due_of = column(roles.due);
    let team_of = column(roles.group);
    let n = table.rows.len();

    let order = settings.date_order.unwrap_or_else(|| {
        date::detect_order(
            (0..n)
                .flat_map(|r| [created_of(r), resolved_of(r), due_of(r)])
                .filter(|s| !s.is_empty()),
        )
    });
    let Some(created_col) = roles.created else {
        return Err((roles, order, Problem::NoCreatedColumn));
    };

    // Status groups: the guess, unless the person moved the status.
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for r in 0..n {
        *counts.entry(status_of(r).trim().to_string()).or_insert(0) += 1;
    }
    let mut statuses: Vec<StatusUse> = counts
        .into_iter()
        .map(|(name, count)| {
            let guessed = status::guess(&name);
            let chosen = settings.statuses.get(&name).copied().unwrap_or(guessed);
            StatusUse {
                name,
                count,
                group: chosen,
                moved: chosen != guessed,
            }
        })
        .collect();
    statuses.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.name.cmp(&b.name)));
    let group_of: BTreeMap<&str, Group> = statuses.iter().map(|s| (s.name.as_str(), s.group)).collect();

    let mut reading = Reading {
        rows: n,
        ..Reading::default()
    };
    let mut items = Vec::new();
    for r in 0..n {
        let text = created_of(r);
        let Some(created) = date::parse(text, order) else {
            reading.skipped_no_date += 1;
            if reading.first_skipped.is_none() {
                reading.first_skipped = Some((r + 2, text.trim().to_string()));
            }
            continue;
        };
        let group = group_of.get(status_of(r).trim()).copied().unwrap_or(Group::Requested);
        let resolved = if group == Group::Done {
            let day = date::parse(resolved_of(r), order);
            if roles.resolved.is_some() && day.is_none() {
                reading.done_without_date += 1;
            }
            if day.map(|d| d < created).unwrap_or(false) {
                reading.resolved_before_created += 1;
            }
            day.map(|d| d.max(created))
        } else {
            None
        };
        items.push(Item {
            created,
            group,
            resolved,
            due: date::parse(due_of(r), order),
            team: team_of(r).trim().to_string(),
        });
    }
    if items.is_empty() {
        return Err((
            roles,
            order,
            Problem::NoDates {
                column: table.header[created_col].clone(),
            },
        ));
    }

    let mut teams: BTreeMap<String, usize> = BTreeMap::new();
    if roles.group.is_some() {
        for it in &items {
            *teams.entry(it.team.clone()).or_insert(0) += 1;
        }
    }
    let mut teams: Vec<(String, usize)> = teams.into_iter().collect();
    teams.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    if roles.group.is_some() && !settings.only_group.is_empty() {
        let before = items.len();
        items.retain(|it| it.team == settings.only_group);
        reading.other_group = before - items.len();
    }

    let latest = items
        .iter()
        .flat_map(|it| [Some(it.created), it.resolved])
        .flatten()
        .max()
        .unwrap_or(0);
    let as_of = settings.as_of(order).unwrap_or(latest);
    let before = items.len();
    items.retain(|it| it.created <= as_of);
    reading.after_as_of = before - items.len();
    for it in items.iter_mut() {
        if it.resolved.map(|d| d > as_of).unwrap_or(false) {
            // Completed after the as-of day: open on that day.
            it.resolved = None;
            it.group = Group::Pending;
        }
    }
    reading.used = items.len();

    Ok(Data {
        has_resolved: roles.resolved.is_some(),
        has_due: roles.due.is_some(),
        roles,
        order,
        statuses,
        teams,
        items,
        as_of,
        reading,
    })
}

/// The counts for one period.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Row {
    pub index: i64,
    pub label: String,
    pub short: String,
    /// The period holds the as-of day and has days after it.
    pub partial: bool,
    /// Items created in the period, all groups.
    pub received: i64,
    /// Of those, by today's status: requested, pending, done, excluded.
    pub requested: i64,
    pub pending: i64,
    pub done: i64,
    pub excluded: i64,
    /// Items completed in the period (by completion date).
    pub completed: Option<i64>,
    /// Percent of the period's items, excluded ones aside, that are done.
    pub rate: Option<i64>,
    /// Items open at the end of the period (or on the as-of day).
    pub open_end: Option<i64>,
    /// Median days from created to completed, for items completed in it.
    pub median_days: Option<i64>,
    /// Percent change in `received` on the period before, and on the same
    /// period a year before.
    pub change_prev: Option<i64>,
    pub change_year: Option<i64>,
}

impl Row {
    pub fn count(&self, g: Group) -> i64 {
        match g {
            Group::Requested => self.requested,
            Group::Pending => self.pending,
            Group::Done => self.done,
            Group::Excluded => self.excluded,
        }
    }

    /// Items received that count toward completion.
    pub fn counted(&self) -> i64 {
        self.received - self.excluded
    }
}

/// The percent change from `old` to `new`, rounded half away from zero.
/// `None` when there is nothing to compare with.
pub fn change(new: i64, old: i64) -> Option<i64> {
    if old <= 0 {
        return None;
    }
    Some(divide_rounded((new - old) * 100, old))
}

/// `part` as a whole percent of `whole`, rounded half away from zero.
pub fn percent(part: i64, whole: i64) -> Option<i64> {
    if whole <= 0 {
        return None;
    }
    Some(divide_rounded(part * 100, whole))
}

/// `a / b` for `b > 0`, rounded half away from zero.
pub fn divide_rounded(a: i64, b: i64) -> i64 {
    let q = a / b;
    let r = a % b;
    if 2 * r.abs() >= b {
        q + a.signum()
    } else {
        q
    }
}

/// The median of whole numbers; for an even count, the mean of the middle
/// two rounded down.
pub fn median(values: &mut [i64]) -> Option<i64> {
    if values.is_empty() {
        return None;
    }
    values.sort_unstable();
    let mid = values.len() / 2;
    if values.len() % 2 == 1 {
        Some(values[mid])
    } else {
        Some((values[mid - 1] + values[mid]).div_euclid(2))
    }
}

/// One row per period from the first item's period to the as-of day's.
pub fn rows(data: &Data, settings: &Settings) -> Vec<Row> {
    let cal = settings.calendar;
    let first = data
        .items
        .iter()
        .map(|it| cal.index(it.created))
        .min()
        .unwrap_or_else(|| cal.index(data.as_of));
    let last = cal.index(data.as_of);
    let mut rows: Vec<Row> = (first..=last)
        .map(|index| Row {
            index,
            label: cal.label(index),
            short: cal.short_label(index),
            partial: index == last && data.as_of + 1 < cal.end(index),
            received: 0,
            requested: 0,
            pending: 0,
            done: 0,
            excluded: 0,
            completed: data.has_resolved.then_some(0),
            rate: None,
            open_end: None,
            median_days: None,
            change_prev: None,
            change_year: None,
        })
        .collect();
    let at = |index: i64| (index - first) as usize;
    let mut days: Vec<Vec<i64>> = vec![Vec::new(); rows.len()];
    for it in &data.items {
        let row = &mut rows[at(cal.index(it.created))];
        row.received += 1;
        match it.group {
            Group::Requested => row.requested += 1,
            Group::Pending => row.pending += 1,
            Group::Done => row.done += 1,
            Group::Excluded => row.excluded += 1,
        }
        if let Some(r) = it.resolved {
            let i = at(cal.index(r));
            if let Some(c) = rows[i].completed.as_mut() {
                *c += 1;
            }
            days[i].push(r - it.created);
        }
    }
    let per_year = cal.per_year();
    for i in 0..rows.len() {
        let end = if rows[i].partial {
            data.as_of + 1
        } else {
            cal.end(rows[i].index)
        };
        if data.has_resolved {
            let open = data
                .items
                .iter()
                .filter(|it| it.group != Group::Excluded && it.created < end)
                .filter(|it| match (it.group, it.resolved) {
                    (Group::Done, Some(r)) => r >= end,
                    // Done with no completion date: when is unknown, so
                    // it is not counted as open at any period's end.
                    (Group::Done, None) => false,
                    _ => true,
                })
                .count() as i64;
            rows[i].open_end = Some(open);
            rows[i].median_days = median(&mut days[i]);
        }
        rows[i].rate = percent(rows[i].done, rows[i].counted());
        if i >= 1 {
            rows[i].change_prev = change(rows[i].received, rows[i - 1].received);
        }
        if i as i64 >= per_year {
            rows[i].change_year = change(rows[i].received, rows[i - per_year as usize].received);
        }
    }
    rows
}

/// The last period that is not in progress, or the last period when every
/// period is (the file covers part of one period).
pub fn last_complete(rows: &[Row]) -> usize {
    rows.iter()
        .rposition(|r| !r.partial)
        .unwrap_or(rows.len().saturating_sub(1))
}

/// Open items on the as-of day, and how many are past due or old.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenNow {
    pub open: i64,
    pub overdue: i64,
    pub old: i64,
}

pub fn open_now(data: &Data, aging: i64) -> OpenNow {
    let open: Vec<&Item> = data.items.iter().filter(|it| it.group.is_open()).collect();
    OpenNow {
        open: open.len() as i64,
        overdue: open
            .iter()
            .filter(|it| it.due.map(|d| d < data.as_of).unwrap_or(false))
            .count() as i64,
        old: open.iter().filter(|it| data.as_of - it.created > aging).count() as i64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_changes_round_half_away_from_zero() {
        assert_eq!(change(4, 3), Some(33));
        assert_eq!(change(2, 3), Some(-33));
        assert_eq!(change(3, 2), Some(50));
        assert_eq!(change(1, 8), Some(-88)); // -87.5
        assert_eq!(change(15, 8), Some(88)); // +87.5
        assert_eq!(change(5, 5), Some(0));
        assert_eq!(change(5, 0), None);
        assert_eq!(percent(2, 3), Some(67));
        assert_eq!(percent(1, 2), Some(50));
        assert_eq!(percent(1, 0), None);
    }

    #[test]
    fn medians() {
        assert_eq!(median(&mut []), None);
        assert_eq!(median(&mut [9, 1, 5]), Some(5));
        assert_eq!(median(&mut [20, 10]), Some(15));
        assert_eq!(median(&mut [7, 28]), Some(17));
    }
}
