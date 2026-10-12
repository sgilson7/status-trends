//! Reporting periods: years, quarters or months, with a fiscal year that may
//! start in any month. A period is a whole-number index, so neighbouring
//! periods differ by one and the same period a year before is `per_year`
//! back.
//!
//! A fiscal year is named for the calendar year it ends in: with an October
//! start, FY26 runs from October 2025 to September 2026.

use serde::Deserialize;

use crate::date::{self, Day, MONTH_NAMES};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Granularity {
    Year,
    Quarter,
    Month,
}

impl Granularity {
    pub fn from_key(key: &str) -> Option<Granularity> {
        match key {
            "year" => Some(Granularity::Year),
            "quarter" => Some(Granularity::Quarter),
            "month" => Some(Granularity::Month),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Calendar {
    pub granularity: Granularity,
    /// 1 for January to 12 for December.
    pub fiscal_start: i64,
}

/// Calendar months since year 0: January of year y is 12 * y.
fn month_number(day: Day) -> i64 {
    let (y, m, _) = date::civil_from_days(day);
    y * 12 + m - 1
}

fn first_day_of_month(month_number: i64) -> Day {
    date::days_from_civil(
        month_number.div_euclid(12),
        month_number.rem_euclid(12) + 1,
        1,
    )
}

impl Calendar {
    fn fiscal(&self) -> bool {
        self.fiscal_start != 1
    }

    /// Months counted from the start of the fiscal year 0.
    fn fiscal_month(&self, day: Day) -> i64 {
        month_number(day) - (self.fiscal_start - 1)
    }

    pub fn per_year(&self) -> i64 {
        match self.granularity {
            Granularity::Year => 1,
            Granularity::Quarter => 4,
            Granularity::Month => 12,
        }
    }

    fn months_per_period(&self) -> i64 {
        12 / self.per_year()
    }

    /// The period a day falls in.
    pub fn index(&self, day: Day) -> i64 {
        self.fiscal_month(day).div_euclid(self.months_per_period())
    }

    /// The first day of a period.
    pub fn start(&self, index: i64) -> Day {
        first_day_of_month(index * self.months_per_period() + self.fiscal_start - 1)
    }

    /// The first day after a period.
    pub fn end(&self, index: i64) -> Day {
        self.start(index + 1)
    }

    /// The year label of the fiscal or calendar year a period is in.
    fn year_name(&self, index: i64, short: bool) -> String {
        let y = index.div_euclid(self.per_year()) + if self.fiscal() { 1 } else { 0 };
        if self.fiscal() {
            format!("FY{:02}", y.rem_euclid(100))
        } else if short {
            format!("\u{2019}{:02}", y.rem_euclid(100))
        } else {
            format!("{y}")
        }
    }

    /// The position of a period inside its year, from 0.
    pub fn position(&self, index: i64) -> i64 {
        index.rem_euclid(self.per_year())
    }

    /// The fiscal or calendar year a period is in, as a whole number that
    /// increases by one each year.
    pub fn year_of(&self, index: i64) -> i64 {
        index.div_euclid(self.per_year())
    }

    /// The year's label, as used in the year-over-year chart's legend.
    pub fn year_label(&self, index: i64) -> String {
        self.year_name(index, false)
    }

    /// The label of a position inside a year: "Q1" or "Jan".
    pub fn position_label(&self, position: i64) -> String {
        match self.granularity {
            Granularity::Year => String::new(),
            Granularity::Quarter => format!("Q{}", position + 1),
            Granularity::Month => MONTH_NAMES
                [((position + self.fiscal_start - 1).rem_euclid(12)) as usize]
                .to_string(),
        }
    }

    /// The full label: "2025", "FY26", "2025 Q3", "FY26 Q1", "Mar 2025".
    pub fn label(&self, index: i64) -> String {
        match self.granularity {
            Granularity::Year => self.year_name(index * self.per_year(), false),
            Granularity::Quarter => {
                format!(
                    "{} {}",
                    self.year_name(index, false),
                    self.position_label(self.position(index))
                )
            }
            Granularity::Month => {
                let m = index + self.fiscal_start - 1;
                format!(
                    "{} {}",
                    MONTH_NAMES[m.rem_euclid(12) as usize],
                    m.div_euclid(12)
                )
            }
        }
    }

    /// A short label for a chart axis: "2025", "Q3 ’25", "Mar ’25".
    pub fn short_label(&self, index: i64) -> String {
        match self.granularity {
            Granularity::Year => self.label(index),
            Granularity::Quarter if self.fiscal() => self.label(index),
            Granularity::Quarter => {
                format!(
                    "{} {}",
                    self.position_label(self.position(index)),
                    self.year_name(index, true)
                )
            }
            Granularity::Month => {
                let m = index + self.fiscal_start - 1;
                format!(
                    "{} \u{2019}{:02}",
                    MONTH_NAMES[m.rem_euclid(12) as usize],
                    m.div_euclid(12).rem_euclid(100)
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::date::days_from_civil as d;

    fn cal(g: Granularity, start: i64) -> Calendar {
        Calendar {
            granularity: g,
            fiscal_start: start,
        }
    }

    #[test]
    fn calendar_quarters() {
        let c = cal(Granularity::Quarter, 1);
        let q = c.index(d(2025, 3, 31));
        assert_eq!(c.index(d(2025, 1, 1)), q);
        assert_eq!(c.index(d(2025, 4, 1)), q + 1);
        assert_eq!(c.start(q + 1), d(2025, 4, 1));
        assert_eq!(c.end(q), d(2025, 4, 1));
        assert_eq!(c.label(q), "2025 Q1");
        assert_eq!(c.short_label(q), "Q1 \u{2019}25");
        assert_eq!(c.index(d(2026, 1, 15)) - q, 4);
    }

    #[test]
    fn an_october_fiscal_year_is_named_for_the_year_it_ends() {
        let y = cal(Granularity::Year, 10);
        assert_eq!(y.index(d(2025, 10, 1)), y.index(d(2026, 9, 30)));
        assert_ne!(y.index(d(2025, 9, 30)), y.index(d(2025, 10, 1)));
        assert_eq!(y.label(y.index(d(2026, 9, 30))), "FY26");
        assert_eq!(y.start(y.index(d(2026, 3, 1))), d(2025, 10, 1));

        let q = cal(Granularity::Quarter, 10);
        let i = q.index(d(2025, 11, 20));
        assert_eq!(q.label(i), "FY26 Q1");
        assert_eq!(q.start(i), d(2025, 10, 1));
        assert_eq!(q.end(i), d(2026, 1, 1));
        assert_eq!(q.label(q.index(d(2026, 9, 1))), "FY26 Q4");
        assert_eq!(q.label(q.index(d(2026, 10, 1))), "FY27 Q1");
    }

    #[test]
    fn months_keep_calendar_names_and_fiscal_positions() {
        let m = cal(Granularity::Month, 10);
        let i = m.index(d(2025, 10, 31));
        assert_eq!(m.label(i), "Oct 2025");
        assert_eq!(m.short_label(i), "Oct \u{2019}25");
        assert_eq!(m.position(i), 0);
        assert_eq!(m.position_label(0), "Oct");
        assert_eq!(m.position_label(11), "Sep");
        assert_eq!(m.label(i + 3), "Jan 2026");
        assert_eq!(m.year_label(i), m.year_label(i + 11));
        assert_eq!(m.year_label(i), "FY26");
        assert_eq!(m.start(i + 3), d(2026, 1, 1));
        let c = cal(Granularity::Month, 1);
        assert_eq!(c.label(c.index(d(2024, 2, 29))), "Feb 2024");
    }

    #[test]
    fn years_and_positions_step_by_one() {
        let c = cal(Granularity::Year, 1);
        let i = c.index(d(2025, 6, 1));
        assert_eq!(c.label(i), "2025");
        assert_eq!(c.label(i + 1), "2026");
        assert_eq!(c.per_year(), 1);
        let q = cal(Granularity::Quarter, 1);
        assert_eq!(
            q.year_of(q.index(d(2025, 12, 31))) + 1,
            q.year_of(q.index(d(2026, 1, 1)))
        );
        assert_eq!(q.year_label(q.index(d(2025, 5, 5))), "2025");
        assert_eq!(Granularity::from_key("month"), Some(Granularity::Month));
        assert_eq!(Granularity::from_key("week"), None);
    }
}
