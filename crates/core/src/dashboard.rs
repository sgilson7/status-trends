//! Everything the page draws for one file and one set of settings: the
//! column and status choices, the tiles, the insights, the charts, the
//! tables and the CSV export. One call, one value.

use serde::Serialize;

use crate::analysis::{self, change, percent, Data, OpenNow, Row};
use crate::chart::{self, thousands, Chart, Legend, Lines, Series, Stacked};
use crate::columns::Roles;
use crate::csv::Table;
use crate::date::{self, NumericOrder};
use crate::insights::{self, signed, GroupRate, Insight};
use crate::period::Granularity;
use crate::settings::Settings;
use crate::status::{Group, STACK_ORDER};
use crate::strings::{Strings, Vocabulary};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Column {
    pub index: usize,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Delta {
    pub text: String,
    /// "good", "bad" or "neutral": the colour of the change.
    pub tone: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Tile {
    pub label: String,
    pub value: String,
    pub detail: String,
    pub deltas: Vec<Delta>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TableView {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    /// Rows to mark as in progress.
    pub partial: Vec<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Summary {
    pub text: String,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct View {
    pub columns: Vec<Column>,
    pub roles: Roles,
    /// "month" or "day": how numeric dates were read.
    pub date_order: &'static str,
    pub statuses: Vec<analysis::StatusUse>,
    /// Group values for the "show only" menu, most items first.
    pub groups: Vec<String>,
    /// Set when there is no dashboard yet; the page shows it.
    pub problem: Option<String>,
    pub summary: Option<Summary>,
    pub tiles: Vec<Tile>,
    pub insights: Vec<Insight>,
    pub charts: Vec<Chart>,
    pub table: Option<TableView>,
    pub group_table: Option<TableView>,
    pub export_csv: String,
    pub export_name: String,
}

/// The most group values listed in the menu and the group table.
pub const MAX_GROUPS: usize = 50;
/// The most periods drawn; with more, the latest are shown.
pub const MAX_SHOWN: usize = 120;

fn order_key(o: NumericOrder) -> &'static str {
    match o {
        NumericOrder::MonthFirst => "month",
        NumericOrder::DayFirst => "day",
    }
}

fn period_word(g: Granularity) -> &'static str {
    match g {
        Granularity::Year => "year",
        Granularity::Quarter => "quarter",
        Granularity::Month => "month",
    }
}

fn opt(v: Option<i64>, f: impl Fn(i64) -> String) -> String {
    v.map(f).unwrap_or_else(|| "\u{2014}".to_string())
}

fn series_class(g: Group) -> &'static str {
    match g {
        Group::Requested => "s-requested",
        Group::Pending => "s-pending",
        Group::Done => "s-done",
        Group::Excluded => "s-excluded",
    }
}

fn tone(change: i64, up_is_good: Option<bool>) -> &'static str {
    match (change.signum(), up_is_good) {
        (0, _) | (_, None) => "neutral",
        (1, Some(true)) | (-1, Some(false)) => "good",
        _ => "bad",
    }
}

/// Builds the whole view. Errors in the file itself come back as `Err`.
pub fn build(table: &Table, settings: &mut Settings, strings: &Strings) -> View {
    let words = strings.vocabulary(&settings.vocabulary).clone();
    let columns: Vec<Column> = table
        .header
        .iter()
        .enumerate()
        .map(|(index, name)| Column {
            index,
            name: name.clone(),
        })
        .collect();
    let data = match analysis::read(table, settings) {
        Ok(d) => d,
        Err((roles, order, problem)) => {
            return View {
                columns,
                roles,
                date_order: order_key(order),
                statuses: Vec::new(),
                groups: Vec::new(),
                problem: Some(problem.message()),
                summary: None,
                tiles: Vec::new(),
                insights: Vec::new(),
                charts: Vec::new(),
                table: None,
                group_table: None,
                export_csv: String::new(),
                export_name: String::new(),
            }
        }
    };
    let all_rows = analysis::rows(&data, settings);
    let last = analysis::last_complete(&all_rows);
    let open = analysis::open_now(&data, settings.aging);
    let as_of_label = date::label(data.as_of);
    let group_rates = group_rates(&data, settings, &all_rows);
    let found = insights::find(&insights::Context {
        rows: &all_rows,
        last,
        open,
        as_of_label: as_of_label.clone(),
        has_resolved: data.has_resolved,
        has_due: data.has_due,
        groups: &group_rates,
        settings,
        words: &words,
    });
    let shown_from = settings
        .show
        .map(|n| all_rows.len().saturating_sub(n))
        .unwrap_or(0)
        .max(all_rows.len().saturating_sub(MAX_SHOWN));
    let shown = &all_rows[shown_from..];
    let word = period_word(settings.calendar.granularity);
    let summary = summary(&data, &words, &as_of_label, table, settings);
    let tiles = tiles(&all_rows, last, open, &data, settings, &words);
    let charts = charts(shown, &all_rows, shown_from, &data, settings, &words, word);
    let period_table = period_table(shown, &data, &words, settings);
    let export_csv = to_csv(&period_table);
    let group_table = (data.roles.group.is_some() && settings.only_group.is_empty())
        .then(|| group_table(&data, &group_rates, &all_rows, last, settings, table));
    View {
        columns,
        roles: data.roles.clone(),
        date_order: order_key(data.order),
        statuses: data.statuses.clone(),
        groups: data.teams.iter().take(MAX_GROUPS).map(|(n, _)| n.clone()).collect(),
        problem: None,
        summary: Some(summary),
        tiles,
        insights: found,
        charts,
        table: Some(period_table),
        group_table,
        export_csv,
        export_name: format!("status-trends-by-{word}.csv"),
    }
}

fn summary(data: &Data, w: &Vocabulary, as_of: &str, table: &Table, settings: &Settings) -> Summary {
    let r = &data.reading;
    let first = data.items.iter().map(|i| i.created).min().unwrap_or(data.as_of);
    let text = format!(
        "{} {} from {} to {}, read from {} rows.",
        thousands(r.used as i64),
        if r.used == 1 { &w.item } else { &w.items },
        date::label(first),
        as_of,
        thousands(r.rows as i64)
    );
    let mut notes = settings.notes.clone();
    if r.skipped_no_date > 0 {
        let (row, value) = r.first_skipped.clone().unwrap_or((0, String::new()));
        let column = data.roles.created.map(|c| table.header[c].clone()).unwrap_or_default();
        notes.push(format!(
            "{} rows were skipped because the \"{column}\" date could not be read (the first is row {row}: \"{value}\").",
            thousands(r.skipped_no_date as i64)
        ));
    }
    if r.other_group > 0 {
        notes.push(format!(
            "Showing only \"{}\": {} other rows are set aside.",
            settings.only_group,
            thousands(r.other_group as i64)
        ));
    }
    if r.after_as_of > 0 {
        notes.push(format!(
            "{} rows created after {as_of} are left out.",
            thousands(r.after_as_of as i64)
        ));
    }
    if r.done_without_date > 0 {
        notes.push(format!(
            "{} completed {} have no completion date, so they count as completed but not in any period's completed count.",
            thousands(r.done_without_date as i64),
            w.items
        ));
    }
    if r.resolved_before_created > 0 {
        notes.push(format!(
            "{} {} have a completion date before their created date; they are counted as completed the day they were created.",
            thousands(r.resolved_before_created as i64),
            w.items
        ));
    }
    if !data.has_resolved {
        notes.push("No completion-date column is chosen, so the charts of completed and open items over time are not drawn.".to_string());
    }
    Summary { text, notes }
}

fn tiles(rows: &[Row], last: usize, open: OpenNow, data: &Data, s: &Settings, w: &Vocabulary) -> Vec<Tile> {
    let l = &rows[last];
    let prev = last.checked_sub(1).map(|i| &rows[i]);
    let year_back = last
        .checked_sub(s.calendar.per_year() as usize)
        .filter(|_| s.calendar.per_year() > 1)
        .map(|i| &rows[i]);
    let mut out = Vec::new();

    let mut deltas = Vec::new();
    if let (Some(p), Some(c)) = (prev, l.change_prev) {
        deltas.push(Delta { text: format!("{} on {}", signed(c), p.label), tone: "neutral" });
    }
    if let (Some(y), Some(c)) = (year_back, l.change_year) {
        deltas.push(Delta { text: format!("{} on {}", signed(c), y.label), tone: "neutral" });
    }
    out.push(Tile {
        label: format!("{} in {}", w.received, l.label),
        value: thousands(l.received),
        detail: format!("{} {}, {} {}", thousands(l.done), w.done.to_lowercase(), thousands(l.count(Group::Requested) + l.count(Group::Pending)), "still open"),
        deltas,
    });

    let mut deltas = Vec::new();
    if let (Some(p), Some(now), Some(before)) = (prev, l.rate, prev.and_then(|p| p.rate)) {
        let d = now - before;
        let sign = if d > 0 { "+" } else if d < 0 { "\u{2212}" } else { "" };
        deltas.push(Delta { text: format!("{sign}{} pts on {}", d.abs(), p.label), tone: tone(d, Some(true)) });
    }
    out.push(Tile {
        label: format!("Completed, of {} received in {}", w.items, l.label),
        value: opt(l.rate, |r| format!("{r}%")),
        detail: format!(
            "Target {}%{}",
            s.target,
            match l.rate {
                Some(r) if r < s.target => ", below target",
                Some(_) => ", on target",
                None => "",
            }
        ),
        deltas,
    });

    if data.has_resolved {
        let mut deltas = Vec::new();
        if let (Some(p), Some(now), Some(before)) = (prev, l.completed, prev.and_then(|p| p.completed)) {
            if let Some(c) = change(now, before) {
                deltas.push(Delta { text: format!("{} on {}", signed(c), p.label), tone: tone(c, Some(true)) });
            }
        }
        out.push(Tile {
            label: format!("{} in {}", w.done, l.label),
            value: opt(l.completed, thousands),
            detail: format!("Median {} to complete", opt(l.median_days, |d| format!("{d} days"))),
            deltas: {
                if let (Some(p), Some(now), Some(before)) = (prev, l.median_days, prev.and_then(|p| p.median_days)) {
                    if let Some(c) = change(now, before) {
                        deltas.push(Delta { text: format!("median {} on {}", signed(c), p.label), tone: tone(c, Some(false)) });
                    }
                }
                deltas
            },
        });
    }

    let mut deltas = Vec::new();
    if let Some(before) = l.open_end.filter(|_| rows.last().map(|r| r.partial).unwrap_or(false)) {
        if let Some(c) = change(open.open, before) {
            deltas.push(Delta { text: format!("{} since the end of {}", signed(c), l.label), tone: tone(c, Some(false)) });
        }
    }
    let mut detail = Vec::new();
    if data.has_due {
        detail.push(format!("{} past due", thousands(open.overdue)));
    }
    detail.push(format!("{} open over {} days", thousands(open.old), s.aging));
    out.push(Tile {
        label: format!("Open {} on {}", w.items, date::label(data.as_of)),
        value: thousands(open.open),
        detail: detail.join(", "),
        deltas,
    });
    out
}

#[allow(clippy::too_many_arguments)]
fn charts(shown: &[Row], all: &[Row], shown_from: usize, data: &Data, s: &Settings, w: &Vocabulary, word: &str) -> Vec<Chart> {
    let mut out = Vec::new();
    let labels: Vec<String> = shown.iter().map(|r| r.short.clone()).collect();
    let prev_label = |i: usize| (shown_from + i).checked_sub(1).map(|j| all[j].label.clone());

    // Received, by status today.
    let tips: Vec<Vec<String>> = shown
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let mut t = vec![
                format!("{}{}", r.label, if r.partial { " (in progress)" } else { "" }),
                format!("{}: {}", w.received, thousands(r.received)),
            ];
            for g in [Group::Requested, Group::Pending, Group::Done, Group::Excluded] {
                if r.count(g) > 0 || g != Group::Excluded {
                    t.push(format!("{}: {}", w.group_label(g), thousands(r.count(g))));
                }
            }
            if let (Some(c), Some(p)) = (r.change_prev, prev_label(i)) {
                t.push(format!("{} on {p}", signed(c)));
            }
            t
        })
        .collect();
    let any_excluded = shown.iter().any(|r| r.excluded > 0);
    out.push(chart::stacked(Stacked {
        key: "received",
        title: format!("{} per {word}, by status today", w.received),
        note: format!("Each column counts the {} received in that {word}, split by their status today. The figure over a column is its change on the {word} before.", w.items),
        aria: format!("Column chart of {} received per {word}, stacked by status.", w.items),
        legend: STACK_ORDER
            .iter()
            .filter(|g| **g != Group::Excluded || any_excluded)
            .map(|g| Legend { class: series_class(*g).to_string(), label: w.group_label(*g).to_string() })
            .collect(),
        x_labels: labels.clone(),
        stacks: shown
            .iter()
            .map(|r| STACK_ORDER.iter().map(|g| (series_class(*g), r.count(*g))).collect())
            .collect(),
        partial: shown.iter().map(|r| r.partial).collect(),
        labels_above: shown
            .iter()
            .map(|r| if r.partial { Some("so far".to_string()) } else { r.change_prev.map(signed) })
            .collect(),
        tips,
    }));

    // Received and completed.
    if data.has_resolved {
        out.push(chart::lines(Lines {
            key: "flow",
            title: format!("Received and completed per {word}"),
            note: format!("Received counts {} by the day they came in; completed counts them by the day they were finished, whenever they came in.", w.items),
            aria: format!("Line chart of {} received and completed per {word}.", w.items),
            x_labels: labels.clone(),
            series: vec![
                Series { class: "s-received".into(), label: "Received".into(), values: shown.iter().map(|r| Some(r.received)).collect(), area: false },
                Series { class: "s-done".into(), label: w.done.clone(), values: shown.iter().map(|r| r.completed).collect(), area: false },
            ],
            tips: shown
                .iter()
                .map(|r| vec![
                    r.label.clone(),
                    format!("Received: {}", thousands(r.received)),
                    format!("{}: {}", w.done, opt(r.completed, thousands)),
                ])
                .collect(),
            fixed: None,
            target: None,
            dots_below_target: false,
        }));
    }

    // Completion rate against the target.
    out.push(chart::lines(Lines {
        key: "rate",
        title: format!("Completion rate per {word}"),
        note: format!("The share of each {word}'s {} that are completed today, leaving out {}. Recent {word}s are lower while their {} are still being worked on.", w.items, w.excluded.to_lowercase(), w.items),
        aria: format!("Line chart of the completion rate per {word} against a {}% target.", s.target),
        x_labels: labels.clone(),
        series: vec![Series { class: "s-rate".into(), label: "Completion rate".into(), values: shown.iter().map(|r| r.rate).collect(), area: false }],
        tips: shown
            .iter()
            .map(|r| vec![
                r.label.clone(),
                format!("Completed: {}", opt(r.rate, |v| format!("{v}%"))),
                format!("{} of {}", thousands(r.done), thousands(r.counted())),
                match r.rate {
                    Some(v) if v < s.target => format!("Below the {}% target", s.target),
                    Some(_) => format!("Meets the {}% target", s.target),
                    None => String::new(),
                },
            ].into_iter().filter(|t| !t.is_empty()).collect())
            .collect(),
        fixed: Some((100, 25, "%")),
        target: Some((s.target, format!("Target {}%", s.target))),
        dots_below_target: true,
    }));

    // Open at the end of each period.
    if data.has_resolved {
        out.push(chart::lines(Lines {
            key: "open",
            title: format!("Open at the end of each {word}"),
            note: format!("{} or {} at the end of each {word}; the last point is the as-of day when that {word} is in progress.", w.requested, w.pending.to_lowercase()),
            aria: format!("Line chart of open {} at the end of each {word}.", w.items),
            x_labels: labels.clone(),
            series: vec![Series { class: "s-open".into(), label: "Open".into(), values: shown.iter().map(|r| r.open_end).collect(), area: true }],
            tips: shown
                .iter()
                .map(|r| vec![r.label.clone(), format!("Open: {}", opt(r.open_end, thousands))])
                .collect(),
            fixed: None,
            target: None,
            dots_below_target: false,
        }));
    }

    // Year over year.
    let cal = s.calendar;
    if cal.granularity != Granularity::Year {
        let per_year = cal.per_year();
        let complete: Vec<&Row> = all.iter().filter(|r| !r.partial).collect();
        if let (Some(first), Some(lastr)) = (complete.first(), complete.last()) {
            let (y0, y1) = (cal.year_of(first.index), cal.year_of(lastr.index));
            let years: Vec<i64> = ((y0.max(y1 - 3))..=y1).collect();
            if years.len() >= 2 {
                let oldest = cal.year_of(all[0].index);
                let value = |year: i64, pos: i64| {
                    complete
                        .iter()
                        .find(|r| r.index == year * per_year + pos)
                        .map(|r| r.received)
                };
                let series: Vec<Series> = years
                    .iter()
                    .map(|y| Series {
                        class: format!("s-y{}", (y - oldest).rem_euclid(8) + 1),
                        label: cal.year_label(y * per_year),
                        values: (0..per_year).map(|p| value(*y, p)).collect(),
                        area: false,
                    })
                    .collect();
                out.push(chart::lines(Lines {
                    key: "years",
                    title: format!("Year over year: {} received per {word}", w.items),
                    note: format!("One line per year, complete {word}s only, so the same {word} in different years lines up."),
                    aria: format!("Line chart comparing {} received per {word} across years.", w.items),
                    x_labels: (0..per_year).map(|p| cal.position_label(p)).collect(),
                    tips: (0..per_year)
                        .map(|p| {
                            let mut t = vec![cal.position_label(p)];
                            for y in &years {
                                t.push(format!("{}: {}", cal.year_label(y * per_year), opt(value(*y, p), thousands)));
                            }
                            t
                        })
                        .collect(),
                    series,
                    fixed: None,
                    target: None,
                    dots_below_target: false,
                }));
            }
        }
    }
    out
}

fn period_table(shown: &[Row], data: &Data, w: &Vocabulary, s: &Settings) -> TableView {
    let mut headers = vec![
        "Period".to_string(),
        w.received.clone(),
        w.requested.clone(),
        w.pending.clone(),
        w.done.clone(),
        w.excluded.clone(),
        "Change on period before".to_string(),
    ];
    if s.calendar.per_year() > 1 {
        headers.push("Change on year before".to_string());
    }
    headers.push("Completion rate".to_string());
    if data.has_resolved {
        headers.extend([
            "Completed in period".to_string(),
            "Open at end".to_string(),
            "Median days to complete".to_string(),
        ]);
    }
    let rows = shown
        .iter()
        .map(|r| {
            let mut cells = vec![
                if r.partial { format!("{} (in progress)", r.label) } else { r.label.clone() },
                thousands(r.received),
                thousands(r.requested),
                thousands(r.pending),
                thousands(r.done),
                thousands(r.excluded),
                opt(r.change_prev, signed),
            ];
            if s.calendar.per_year() > 1 {
                cells.push(opt(r.change_year, signed));
            }
            cells.push(opt(r.rate, |v| format!("{v}%")));
            if data.has_resolved {
                cells.push(opt(r.completed, thousands));
                cells.push(opt(r.open_end, thousands));
                cells.push(opt(r.median_days, |d| d.to_string()));
            }
            cells
        })
        .collect();
    TableView {
        headers,
        rows,
        partial: shown.iter().map(|r| r.partial).collect(),
    }
}

/// Each group's completion over the complete periods (the items created
/// before the period in progress).
fn group_rates(data: &Data, s: &Settings, rows: &[Row]) -> Vec<GroupRate> {
    if data.roles.group.is_none() || !s.only_group.is_empty() {
        return Vec::new();
    }
    let cutoff = rows
        .iter()
        .find(|r| r.partial)
        .map(|r| s.calendar.start(r.index))
        .unwrap_or(data.as_of + 1);
    data.teams
        .iter()
        .map(|(name, _)| {
            let items = data
                .items
                .iter()
                .filter(|i| &i.team == name && i.created < cutoff && i.group != Group::Excluded);
            let counted = items.clone().count() as i64;
            let done = items.filter(|i| i.group == Group::Done).count() as i64;
            GroupRate { name: name.clone(), counted, rate: percent(done, counted) }
        })
        .collect()
}

fn group_table(data: &Data, rates: &[GroupRate], rows: &[Row], last: usize, s: &Settings, table: &Table) -> TableView {
    let cal = s.calendar;
    let l = &rows[last];
    let prev = last.checked_sub(1).map(|i| rows[i].index);
    let column = data.roles.group.map(|c| table.header[c].clone()).unwrap_or_default();
    let count_in = |name: &str, index: Option<i64>| {
        index.map(|ix| {
            data.items
                .iter()
                .filter(|i| i.team == name && cal.index(i.created) == ix)
                .count() as i64
        })
    };
    let mut headers = vec![column, format!("Received in {}", l.label)];
    if let Some(p) = last.checked_sub(1) {
        headers.push(format!("Change on {}", rows[p].label));
    }
    headers.extend(["Open now".to_string()]);
    if data.has_due {
        headers.push("Past due".to_string());
    }
    headers.push("Completed, complete periods".to_string());
    let out_rows = rates
        .iter()
        .take(MAX_GROUPS)
        .map(|g| {
            let name = if g.name.is_empty() { "(blank)".to_string() } else { g.name.clone() };
            let now = count_in(&g.name, Some(l.index)).unwrap_or(0);
            let open: Vec<&analysis::Item> = data.items.iter().filter(|i| i.team == g.name && i.group.is_open()).collect();
            let mut cells = vec![name, thousands(now)];
            if prev.is_some() {
                cells.push(opt(count_in(&g.name, prev).and_then(|b| change(now, b)), signed));
            }
            cells.push(thousands(open.len() as i64));
            if data.has_due {
                cells.push(thousands(open.iter().filter(|i| i.due.map(|d| d < data.as_of).unwrap_or(false)).count() as i64));
            }
            let flag = match g.rate {
                Some(r) if r < s.target && g.counted >= insights::GROUP_MINIMUM => " (below target)",
                _ => "",
            };
            cells.push(format!("{}{flag}", opt(g.rate, |r| format!("{r}%"))));
            cells
        })
        .collect::<Vec<_>>();
    TableView {
        headers,
        partial: vec![false; out_rows.len()],
        rows: out_rows,
    }
}

/// A CSV field: quoted when it holds a comma, quote or line break. A
/// leading = or @, or a + or - not followed by a digit, gets an apostrophe
/// so a spreadsheet does not run it as a formula.
fn field(s: &str) -> String {
    let signed_number = s.starts_with(['+', '-']) && s[1..].starts_with(|c: char| c.is_ascii_digit());
    let risky = s.starts_with(['=', '@']) || (s.starts_with(['+', '-']) && !signed_number);
    let s = if risky { format!("'{s}") } else { s.to_string() };
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s
    }
}

pub fn to_csv(t: &TableView) -> String {
    let mut out = String::new();
    for line in std::iter::once(&t.headers).chain(t.rows.iter()) {
        let cells: Vec<String> = line.iter().map(|c| field(&c.replace('\u{2212}', "-").replace('\u{2014}', ""))).collect();
        out.push_str(&cells.join(","));
        out.push_str("\r\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_fields_are_quoted_and_defused() {
        assert_eq!(field("a,b"), "\"a,b\"");
        assert_eq!(field("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(field("=SUM(A1)"), "'=SUM(A1)");
        assert_eq!(field("plain"), "plain");
        let t = TableView { headers: vec!["P".into(), "C".into()], rows: vec![vec!["Q1".into(), "\u{2212}5%".into()], vec!["Q2".into(), "\u{2014}".into()]], partial: vec![false, false] };
        // A minus sign becomes a plain hyphen; an empty dash an empty cell.
        assert_eq!(to_csv(&t), "P,C\r\nQ1,-5%\r\nQ2,\r\n");
        assert_eq!(field("+12%"), "+12%");
        assert_eq!(field("-cmd"), "'-cmd");
    }
}
