//! Insights and risks: rules over the period rows, each giving a severity
//! and a sentence. Thresholds come from the settings. The wording sits
//! beside each rule so a test can read the sentence the person will read.

use serde::Serialize;

use crate::analysis::{change, percent, OpenNow, Row};
use crate::settings::Settings;
use crate::strings::Vocabulary;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Risk,
    Watch,
    Good,
    Note,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Insight {
    pub level: Level,
    pub text: String,
}

/// A group's completion over the complete periods, for the group rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupRate {
    pub name: String,
    pub counted: i64,
    pub rate: Option<i64>,
}

/// The fewest counted items a group needs before the group rule names it.
pub const GROUP_MINIMUM: i64 = 5;
/// The share of open items past the age limit that makes it a risk.
pub const OLD_SHARE_RISK: i64 = 25;
/// How many periods in a row make a trend.
pub const RUN: usize = 3;

pub fn signed(p: i64) -> String {
    match p {
        p if p > 0 => format!("+{p}%"),
        p if p < 0 => format!("\u{2212}{}%", -p),
        _ => "0%".to_string(),
    }
}

fn capital(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

pub struct Context<'a> {
    pub rows: &'a [Row],
    /// Index of the last complete period in `rows`.
    pub last: usize,
    pub open: OpenNow,
    pub as_of_label: String,
    pub has_resolved: bool,
    pub has_due: bool,
    pub groups: &'a [GroupRate],
    pub settings: &'a Settings,
    pub words: &'a Vocabulary,
}

/// Every rule, risks first, then things to watch, good news and notes.
pub fn find(cx: &Context) -> Vec<Insight> {
    let mut out = Vec::new();
    let rows = cx.rows;
    if rows.is_empty() {
        return out;
    }
    let s = cx.settings;
    let w = cx.words;
    let l = &rows[cx.last];
    let prev = cx.last.checked_sub(1).map(|i| &rows[i]);
    let push = |out: &mut Vec<Insight>, level, text: String| out.push(Insight { level, text });

    // In progress: comparisons use the last complete period.
    if let Some(cur) = rows.last().filter(|r| r.partial) {
        if cur.index != l.index {
            push(&mut out, Level::Note, format!(
                "{} is still in progress (data to {}), so comparisons use {}.",
                cur.label, cx.as_of_label, l.label
            ));
        } else {
            push(&mut out, Level::Note, format!(
                "The file covers only part of {}, so there is no complete period to compare yet.",
                cur.label
            ));
        }
    }

    // Completion against the target.
    if let Some(rate) = l.rate {
        if rate < s.target {
            push(&mut out, Level::Risk, format!(
                "{rate}% of the {} received in {} are completed, below the {}% target ({} of {}).",
                w.items, l.label, s.target, l.done, l.counted()
            ));
        } else {
            push(&mut out, Level::Good, format!(
                "{rate}% of the {} received in {} are completed, meeting the {}% target.",
                w.items, l.label, s.target
            ));
        }
    }

    // Completion falling period after period.
    let rated: Vec<&Row> = rows[..=cx.last].iter().filter(|r| r.rate.is_some()).collect();
    if rated.len() >= RUN {
        let run = &rated[rated.len() - RUN..];
        if run.windows(2).all(|p| p[1].rate < p[0].rate) {
            let parts: Vec<String> = run
                .iter()
                .map(|r| format!("{}% in {}", r.rate.unwrap_or(0), r.label))
                .collect();
            push(&mut out, Level::Risk, format!(
                "Completion has fallen {} periods running: {}.",
                RUN,
                parts.join(", ")
            ));
        }
    }

    // More came in than went out, period after period.
    if cx.has_resolved && cx.last + 1 >= RUN {
        let run = &rows[cx.last + 1 - RUN..=cx.last];
        let building = run.iter().all(|r| r.counted() > r.completed.unwrap_or(0));
        let first_open = cx.last.checked_sub(RUN).and_then(|i| rows[i].open_end).unwrap_or(0);
        if building {
            push(&mut out, Level::Risk, format!(
                "More {} came in than were completed in each of the last {} periods; open {} went from {} to {}.",
                w.items, RUN, w.items, first_open, l.open_end.unwrap_or(0)
            ));
        }
    }

    // The open count's change over the last complete period.
    if let (Some(p), Some(now)) = (prev.and_then(|p| p.open_end.map(|o| (p, o))), l.open_end) {
        let (p, before) = p;
        if let Some(c) = change(now, before) {
            if c >= s.growth {
                push(&mut out, Level::Watch, format!(
                    "Open {} rose {}% in {}: {} at the end of {}, {} at the end of {}.",
                    w.items, c, l.label, before, p.label, now, l.label
                ));
            } else if c <= -s.growth {
                push(&mut out, Level::Good, format!(
                    "Open {} fell {}% in {}: {} at the end of {}, {} at the end of {}.",
                    w.items, -c, l.label, before, p.label, now, l.label
                ));
            }
        }
    }

    // Open too long, and past due.
    if cx.open.old > 0 {
        let share = percent(cx.open.old, cx.open.open).unwrap_or(0);
        let level = if share >= OLD_SHARE_RISK { Level::Risk } else { Level::Watch };
        push(&mut out, level, format!(
            "{} of {} open {} ({share}%) have been open longer than {} days.",
            cx.open.old, cx.open.open, w.items, s.aging
        ));
    }
    if cx.has_due && cx.open.overdue > 0 {
        let noun = if cx.open.overdue == 1 { &w.item } else { &w.items };
        let verb = if cx.open.overdue == 1 { "is" } else { "are" };
        push(&mut out, Level::Risk, format!(
            "{} open {noun} {verb} past the due date on {}.",
            cx.open.overdue, cx.as_of_label
        ));
    }

    // Volume: on the period before and on the year before.
    if let (Some(p), Some(c)) = (prev, l.change_prev) {
        volume(&mut out, w, s.growth, c, p, l, "on");
    }
    let per_year = s.calendar.per_year() as usize;
    if per_year > 1 && cx.last >= per_year {
        if let Some(c) = l.change_year {
            volume(&mut out, w, s.growth, c, &rows[cx.last - per_year], l, "on");
        }
    }

    // Time to complete.
    if let (Some(p), Some(now)) = (prev, l.median_days) {
        if let Some(before) = p.median_days {
            if let Some(c) = change(now, before) {
                if c >= s.growth {
                    push(&mut out, Level::Watch, format!(
                        "Completing a {} took a median of {} days in {}, up from {} days in {}.",
                        w.item, now, l.label, before, p.label
                    ));
                } else if c <= -s.growth {
                    push(&mut out, Level::Good, format!(
                        "Completing a {} took a median of {} days in {}, down from {} days in {}.",
                        w.item, now, l.label, before, p.label
                    ));
                }
            }
        }
    }

    // Groups below the target.
    let mut low: Vec<&GroupRate> = cx
        .groups
        .iter()
        .filter(|g| g.counted >= GROUP_MINIMUM && g.rate.map(|r| r < s.target).unwrap_or(false))
        .collect();
    low.sort_by(|a, b| a.rate.cmp(&b.rate).then_with(|| a.name.cmp(&b.name)));
    if !low.is_empty() {
        let named: Vec<String> = low
            .iter()
            .take(RUN)
            .map(|g| format!("{} ({}%)", g.name, g.rate.unwrap_or(0)))
            .collect();
        let more = if low.len() > RUN {
            format!(", and {} more", low.len() - RUN)
        } else {
            String::new()
        };
        push(&mut out, Level::Risk, format!(
            "{} below the {}% target over the complete periods: {}{more}.",
            capital(&format!("{}s", w.group)),
            s.target,
            named.join(", ")
        ));
    }

    if !out.iter().any(|i| matches!(i.level, Level::Risk | Level::Watch)) {
        push(&mut out, Level::Good, "No risks found with these thresholds.".to_string());
    }
    out.sort_by_key(|i| i.level);
    out
}

fn volume(out: &mut Vec<Insight>, w: &Vocabulary, growth: i64, c: i64, base: &Row, l: &Row, on: &str) {
    let text = |dir: &str| {
        format!(
            "{} received {dir} {}% {on} {}: {} in {}, {} in {}.",
            capital(&w.items),
            c.abs(),
            base.label,
            base.received,
            base.label,
            l.received,
            l.label
        )
    };
    if c >= growth {
        out.push(Insight {
            level: Level::Watch,
            text: text("rose"),
        });
    } else if c <= -growth {
        out.push(Insight {
            level: Level::Note,
            text: text("fell"),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::period::{Calendar, Granularity};
    use crate::settings::{Raw, Settings};
    use crate::strings::tests::sample;

    fn row(i: i64, received: i64, done: i64, completed: i64, open_end: i64) -> Row {
        Row {
            index: i,
            label: format!("P{i}"),
            short: format!("P{i}"),
            partial: false,
            received,
            requested: received - done,
            pending: 0,
            done,
            excluded: 0,
            completed: Some(completed),
            rate: percent(done, received),
            open_end: Some(open_end),
            median_days: None,
            change_prev: None,
            change_year: None,
        }
    }

    fn settings() -> Settings {
        let mut s = Settings::read(&Raw::default());
        s.calendar = Calendar {
            granularity: Granularity::Quarter,
            fiscal_start: 1,
        };
        s
    }

    fn run(rows: &[Row], open: OpenNow, groups: &[GroupRate]) -> Vec<Insight> {
        let s = settings();
        let strings = sample();
        let cx = Context {
            rows,
            last: rows.iter().rposition(|r| !r.partial).unwrap_or(0),
            open,
            as_of_label: "5 Apr 2026".into(),
            has_resolved: true,
            has_due: true,
            groups,
            settings: &s,
            words: strings.vocabulary("requests"),
        };
        find(&cx)
    }

    const NONE_OPEN: OpenNow = OpenNow { open: 0, overdue: 0, old: 0 };

    fn texts(found: &[Insight]) -> Vec<String> {
        found.iter().map(|i| format!("{:?}: {}", i.level, i.text)).collect()
    }

    fn has(found: &[Insight], level: Level, text: &str) -> bool {
        found.iter().any(|i| i.level == level && i.text == text)
    }

    #[test]
    fn steady_data_finds_no_risk() {
        let rows = vec![row(0, 10, 9, 9, 1), row(1, 10, 9, 10, 1)];
        let found = run(&rows, NONE_OPEN, &[]);
        assert!(has(&found, Level::Good, "90% of the requests received in P1 are completed, meeting the 80% target."), "{:?}", texts(&found));
        assert!(has(&found, Level::Good, "No risks found with these thresholds."));
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn completion_below_target_is_a_risk() {
        let rows = vec![row(0, 10, 7, 7, 3)];
        let found = run(&rows, NONE_OPEN, &[]);
        assert!(has(&found, Level::Risk, "70% of the requests received in P0 are completed, below the 80% target (7 of 10)."), "{:?}", texts(&found));
    }

    #[test]
    fn completion_falling_three_periods_is_a_risk() {
        let rows = vec![row(0, 10, 9, 9, 1), row(1, 10, 8, 10, 1), row(2, 10, 7, 10, 1)];
        let found = run(&rows, NONE_OPEN, &[]);
        assert!(has(&found, Level::Risk, "Completion has fallen 3 periods running: 90% in P0, 80% in P1, 70% in P2."), "{:?}", texts(&found));
        // Level, not falling: no such risk.
        let rows = vec![row(0, 10, 9, 9, 1), row(1, 10, 8, 10, 1), row(2, 10, 8, 10, 1)];
        assert!(!run(&rows, NONE_OPEN, &[]).iter().any(|i| i.text.starts_with("Completion has fallen")));
    }

    #[test]
    fn a_building_backlog_is_a_risk() {
        let rows = vec![row(0, 10, 9, 9, 2), row(1, 10, 9, 8, 4), row(2, 10, 9, 8, 6), row(3, 10, 9, 8, 8)];
        let found = run(&rows, NONE_OPEN, &[]);
        assert!(has(&found, Level::Risk, "More requests came in than were completed in each of the last 3 periods; open requests went from 2 to 8."), "{:?}", texts(&found));
        assert!(has(&found, Level::Watch, "Open requests rose 33% in P3: 6 at the end of P2, 8 at the end of P3."));
    }

    #[test]
    fn old_and_overdue_items_are_named() {
        let rows = vec![row(0, 10, 9, 9, 5)];
        let found = run(&rows, OpenNow { open: 5, overdue: 2, old: 2 }, &[]);
        assert!(has(&found, Level::Risk, "2 of 5 open requests (40%) have been open longer than 90 days."), "{:?}", texts(&found));
        assert!(has(&found, Level::Risk, "2 open requests are past the due date on 5 Apr 2026."));
        let found = run(&rows, OpenNow { open: 10, overdue: 1, old: 2 }, &[]);
        assert!(has(&found, Level::Watch, "2 of 10 open requests (20%) have been open longer than 90 days."));
        assert!(has(&found, Level::Risk, "1 open request is past the due date on 5 Apr 2026."));
    }

    #[test]
    fn volume_jumps_on_the_period_and_the_year_before() {
        let mut rows: Vec<Row> = (0..6).map(|i| row(i, 10, 9, 9, 1)).collect();
        rows[5].received = 13;
        rows[5].requested = 4;
        rows[5].change_prev = change(13, 10);
        rows[5].change_year = change(13, 10);
        let found = run(&rows, NONE_OPEN, &[]);
        assert!(has(&found, Level::Watch, "Requests received rose 30% on P4: 10 in P4, 13 in P5."), "{:?}", texts(&found));
        assert!(has(&found, Level::Watch, "Requests received rose 30% on P1: 10 in P1, 13 in P5."));
        rows[5].change_prev = Some(-20);
        let found = run(&rows, NONE_OPEN, &[]);
        assert!(found.iter().any(|i| i.level == Level::Note && i.text.contains("fell 20% on P4")), "{:?}", texts(&found));
        rows[5].change_prev = Some(19);
        assert!(!run(&rows, NONE_OPEN, &[]).iter().any(|i| i.text.contains("on P4")));
    }

    #[test]
    fn slower_completion_is_watched() {
        let mut rows = vec![row(0, 10, 9, 9, 1), row(1, 10, 9, 9, 1)];
        rows[0].median_days = Some(10);
        rows[1].median_days = Some(12);
        let found = run(&rows, NONE_OPEN, &[]);
        assert!(has(&found, Level::Watch, "Completing a request took a median of 12 days in P1, up from 10 days in P0."), "{:?}", texts(&found));
        rows[1].median_days = Some(11);
        assert!(!run(&rows, NONE_OPEN, &[]).iter().any(|i| i.text.starts_with("Completing")));
    }

    #[test]
    fn groups_below_target_are_named_lowest_first() {
        let rows = vec![row(0, 10, 9, 9, 1)];
        let g = |name: &str, counted, rate| GroupRate { name: name.into(), counted, rate: Some(rate) };
        let groups = vec![g("Web", 10, 70), g("Data", 6, 50), g("Ops", 4, 10), g("App", 9, 90), g("Pay", 5, 60), g("Mail", 5, 79)];
        let found = run(&rows, NONE_OPEN, &groups);
        assert!(has(&found, Level::Risk, "Teams below the 80% target over the complete periods: Data (50%), Pay (60%), Web (70%), and 1 more."), "{:?}", texts(&found));
    }

    #[test]
    fn a_period_in_progress_is_noted_and_skipped() {
        let mut rows = vec![row(0, 10, 7, 7, 3), row(1, 2, 0, 0, 5)];
        rows[1].partial = true;
        let found = run(&rows, NONE_OPEN, &[]);
        assert!(has(&found, Level::Note, "P1 is still in progress (data to 5 Apr 2026), so comparisons use P0."), "{:?}", texts(&found));
        assert!(found.iter().any(|i| i.text.contains("received in P0")));
    }

    #[test]
    fn signs_use_a_true_minus() {
        assert_eq!(signed(12), "+12%");
        assert_eq!(signed(-5), "\u{2212}5%");
        assert_eq!(signed(0), "0%");
    }
}
