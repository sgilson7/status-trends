//! Calendar days. A day is a whole number: days since 1970-01-01. Times of
//! day are read past and dropped, because every count here is per day or
//! coarser, and whole numbers keep the arithmetic exact.

pub type Day = i64;

const MONTHS: [&str; 12] = [
    "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
];
pub const MONTH_NAMES: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Days since 1970-01-01 for a proleptic Gregorian date (H. Hinnant's method).
pub fn days_from_civil(y: i64, m: i64, d: i64) -> Day {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// The (year, month, day) of a day number.
pub fn civil_from_days(z: Day) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

pub fn days_in_month(y: i64, m: i64) -> i64 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 => 29,
        _ => 28,
    }
}

fn valid(y: i64, m: i64, d: i64) -> Option<Day> {
    if (1..=12).contains(&m) && d >= 1 && d <= days_in_month(y, m) && (1900..=2200).contains(&y)
    {
        Some(days_from_civil(y, m, d))
    } else {
        None
    }
}

/// "12 Oct 2026", as people read it.
pub fn label(day: Day) -> String {
    let (y, m, d) = civil_from_days(day);
    format!("{d} {} {y}", MONTH_NAMES[(m - 1) as usize])
}

/// "2026-10-12".
pub fn iso(day: Day) -> String {
    let (y, m, d) = civil_from_days(day);
    format!("{y:04}-{m:02}-{d:02}")
}

/// How to read a date written all in numbers with the year last.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericOrder {
    MonthFirst,
    DayFirst,
}

fn number(s: &str) -> Option<i64> {
    if s.is_empty() || s.len() > 4 || !s.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

fn month_name(s: &str) -> Option<i64> {
    let lower = s.to_ascii_lowercase();
    if lower.len() < 3 {
        return None;
    }
    MONTHS
        .iter()
        .position(|m| lower.starts_with(m))
        .map(|i| i as i64 + 1)
}

/// A two-digit year is 2000-2069 or 1970-1999.
fn full_year(s: &str) -> Option<i64> {
    let y = number(s)?;
    match s.len() {
        4 => Some(y),
        2 if y < 70 => Some(2000 + y),
        2 => Some(1900 + y),
        _ => None,
    }
}

/// Reads a date, ignoring any time of day after it. Accepts ISO dates
/// (`2026-10-12`, `2026-10-12T13:13:00`, `2026/10/12`), Jira's default
/// (`12/Oct/26 1:13 PM`), `12-Oct-2026`, `12 Oct 2026`, `Oct 12, 2026`, and
/// numeric dates with the year last (`10/12/2026`, read by `order`).
pub fn parse(text: &str, order: NumericOrder) -> Option<Day> {
    let t = text.trim();
    if t.is_empty() {
        return None;
    }
    let words: Vec<&str> = t
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|w| !w.is_empty())
        .collect();
    let first = words[0];
    // ISO: the year first. Cut a "T13:13" time off.
    let first_date = first.split('T').next().unwrap_or(first);
    let parts: Vec<&str> = first_date.split(['-', '/', '.']).collect();
    if parts.len() == 3 {
        if parts[0].len() == 4 {
            return valid(number(parts[0])?, number(parts[1])?, number(parts[2])?);
        }
        let year = full_year(parts[2])?;
        if let Some(m) = month_name(parts[1]) {
            return valid(year, m, number(parts[0])?);
        }
        let a = number(parts[0])?;
        let b = number(parts[1])?;
        if parts[0].len() > 2 || parts[1].len() > 2 {
            return None;
        }
        return match order {
            NumericOrder::MonthFirst => valid(year, a, b),
            NumericOrder::DayFirst => valid(year, b, a),
        };
    }
    if words.len() >= 3 {
        // "12 Oct 2026" or "Oct 12 2026" (the comma is already gone).
        if let (Some(d), Some(m)) = (number(words[0]), month_name(words[1])) {
            return valid(full_year(words[2])?, m, d);
        }
        if let (Some(m), Some(d)) = (month_name(words[0]), number(words[1])) {
            return valid(full_year(words[2])?, m, d);
        }
    }
    None
}

/// Day-first if any numeric date in the sample has a first part above 12
/// and none has a second part above 12; month-first otherwise.
pub fn detect_order<'a>(samples: impl Iterator<Item = &'a str>) -> NumericOrder {
    let mut first_big = false;
    let mut second_big = false;
    for s in samples {
        let first = s.split_whitespace().next().unwrap_or("");
        let parts: Vec<&str> = first.split(['/', '-', '.']).collect();
        if parts.len() != 3 || parts[0].len() > 2 || parts[1].len() > 2 {
            continue;
        }
        if let (Some(a), Some(b)) = (number(parts[0]), number(parts[1])) {
            first_big |= a > 12;
            second_big |= b > 12;
        }
    }
    if first_big && !second_big {
        NumericOrder::DayFirst
    } else {
        NumericOrder::MonthFirst
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use NumericOrder::*;

    fn ymd(s: &str) -> Option<String> {
        parse(s, MonthFirst).map(iso)
    }

    #[test]
    fn day_numbers_match_hand_counts() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(1970, 2, 1), 31);
        // 2000 is a leap year: 366 days from 2000-01-01 to 2001-01-01.
        assert_eq!(days_from_civil(2001, 1, 1) - days_from_civil(2000, 1, 1), 366);
        assert_eq!(days_from_civil(2026, 1, 25) - days_from_civil(2026, 1, 5), 20);
        assert_eq!(civil_from_days(days_from_civil(2024, 2, 29)), (2024, 2, 29));
        assert_eq!(civil_from_days(-1), (1969, 12, 31));
    }

    #[test]
    fn iso_dates_with_and_without_a_time() {
        assert_eq!(ymd("2026-10-12").as_deref(), Some("2026-10-12"));
        assert_eq!(ymd("2026-10-12 13:13").as_deref(), Some("2026-10-12"));
        assert_eq!(ymd("2026-10-12T13:13:00.000+0100").as_deref(), Some("2026-10-12"));
        assert_eq!(ymd("2026/1/5").as_deref(), Some("2026-01-05"));
    }

    #[test]
    fn jira_and_written_month_dates() {
        assert_eq!(ymd("12/Oct/26 1:13 PM").as_deref(), Some("2026-10-12"));
        assert_eq!(ymd("3/Jan/2025 9:00 AM").as_deref(), Some("2025-01-03"));
        assert_eq!(ymd("12-Oct-2026").as_deref(), Some("2026-10-12"));
        assert_eq!(ymd("12 October 2026").as_deref(), Some("2026-10-12"));
        assert_eq!(ymd("Oct 12, 2026").as_deref(), Some("2026-10-12"));
        assert_eq!(ymd("1/jan/99").as_deref(), Some("1999-01-01"));
    }

    #[test]
    fn numeric_dates_follow_the_order() {
        assert_eq!(parse("10/12/2026", MonthFirst).map(iso).as_deref(), Some("2026-10-12"));
        assert_eq!(parse("10/12/2026", DayFirst).map(iso).as_deref(), Some("2026-12-10"));
        assert_eq!(parse("13/12/2026", MonthFirst), None);
        assert_eq!(parse("12.10.26", DayFirst).map(iso).as_deref(), Some("2026-10-12"));
    }

    #[test]
    fn impossible_or_unreadable_dates_are_none() {
        for s in ["", "  ", "2026-02-30", "2025-02-29", "soon", "12/Foo/26", "2026-13-01", "123/1/2026", "1/1/1"] {
            assert_eq!(ymd(s), None, "{s:?}");
        }
        assert_eq!(ymd("2024-02-29").as_deref(), Some("2024-02-29"));
    }

    #[test]
    fn the_order_is_detected_from_a_first_part_above_twelve() {
        assert_eq!(detect_order(["1/2/2026", "25/2/2026"].into_iter()), DayFirst);
        assert_eq!(detect_order(["1/2/2026", "2/25/2026"].into_iter()), MonthFirst);
        assert_eq!(detect_order(["12/Oct/26", "2026-10-12"].into_iter()), MonthFirst);
        assert_eq!(detect_order(std::iter::empty()), MonthFirst);
    }

    #[test]
    fn labels_read_as_people_write_them() {
        assert_eq!(label(days_from_civil(2026, 4, 5)), "5 Apr 2026");
    }
}
