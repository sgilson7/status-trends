//! Charts as lists of SVG shapes in whole-number view-box coordinates. The
//! page creates one element per mark and sets what is here; it computes no
//! position. Colours are class names, so the page's style sheet picks the
//! light or dark step. Every chart carries a tooltip per hit area, and the
//! same numbers are in the table beside it.
//!
//! Mark specs follow the data-visualisation method: columns at most 24
//! wide with a rounded top and a square baseline, a 2-unit surface gap
//! between stacked segments, 2-unit lines, 4-unit radius end dots, hairline
//! grid, one y-axis per chart.

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Mark {
    pub tag: &'static str,
    pub class: String,
    pub attrs: Vec<(&'static str, String)>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub text: String,
    /// Index into the chart's `tips`, for a hit area.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tip: Option<usize>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<Mark>,
}

fn mark(tag: &'static str, class: &str, attrs: Vec<(&'static str, i64)>) -> Mark {
    Mark {
        tag,
        class: class.to_string(),
        attrs: attrs.into_iter().map(|(k, v)| (k, v.to_string())).collect(),
        text: String::new(),
        tip: None,
        children: Vec::new(),
    }
}

fn text(class: &str, x: i64, y: i64, anchor: &'static str, s: String) -> Mark {
    let mut m = mark("text", class, vec![("x", x), ("y", y)]);
    m.attrs.push(("text-anchor", anchor.to_string()));
    m.text = s;
    m
}

fn path(class: &str, d: String) -> Mark {
    let mut m = mark("path", class, vec![]);
    m.attrs.push(("d", d));
    m
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Legend {
    pub class: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Chart {
    pub key: String,
    pub title: String,
    pub note: String,
    pub width: i64,
    pub height: i64,
    pub legend: Vec<Legend>,
    pub marks: Vec<Mark>,
    /// Lines of text for each hit area's tooltip.
    pub tips: Vec<Vec<String>>,
    /// A one-sentence description for screen readers.
    pub aria: String,
}

pub const WIDTH: i64 = 960;
pub const HEIGHT: i64 = 340;
const LEFT: i64 = 56;
const RIGHT: i64 = 20;
const TOP: i64 = 30;
const BOTTOM: i64 = 40;
const PLOT_W: i64 = WIDTH - LEFT - RIGHT;
const PLOT_H: i64 = HEIGHT - TOP - BOTTOM;
const BAR_MAX: i64 = 24;
const GAP: i64 = 2;
const RADIUS: i64 = 4;
const MAX_X_LABELS: i64 = 12;
/// Columns get a change label each up to this many; past it, the last few.
const LABEL_ALL_UP_TO: usize = 16;
const LABEL_LAST: usize = 4;

/// The smallest 1, 2 or 5 times a power of ten that covers `max` in four
/// steps, and the axis top it gives.
pub fn scale(max: i64) -> (i64, i64) {
    let max = max.max(1);
    let mut unit = 1;
    loop {
        for m in [1, 2, 5] {
            let step = m * unit;
            if step * 4 >= max {
                let top = (max + step - 1) / step * step;
                return (step, top);
            }
        }
        unit *= 10;
    }
}

/// Axis numbers: 950, 1K, 1.5K, 12K.
pub fn compact(v: i64) -> String {
    if v.abs() < 1000 {
        return v.to_string();
    }
    let whole = v / 1000;
    let tenth = (v % 1000).abs() / 100;
    if tenth == 0 || v.abs() >= 10000 {
        format!("{whole}K")
    } else {
        format!("{whole}.{tenth}K")
    }
}

/// 1284 as "1,284".
pub fn thousands(v: i64) -> String {
    let digits = v.abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    if v < 0 {
        format!("\u{2212}{out}")
    } else {
        out
    }
}

struct Frame {
    n: i64,
    band: i64,
    start: i64,
    top: i64,
}

impl Frame {
    fn new(n: usize, top: i64) -> Frame {
        let n = (n as i64).max(1);
        let band = PLOT_W / n;
        Frame {
            n,
            band,
            start: LEFT + (PLOT_W - band * n) / 2,
            top: top.max(1),
        }
    }

    fn y(&self, v: i64) -> i64 {
        TOP + PLOT_H - v * PLOT_H / self.top
    }

    fn cx(&self, i: usize) -> i64 {
        self.start + self.band * i as i64 + self.band / 2
    }

    fn bar_w(&self) -> i64 {
        (self.band * 3 / 5).clamp(2, BAR_MAX)
    }

    /// Which x labels to draw: every k-th counting back from the last.
    fn shows_label(&self, i: usize) -> bool {
        let k = (self.n + MAX_X_LABELS - 1) / MAX_X_LABELS;
        (self.n - 1 - i as i64) % k == 0
    }
}

/// Gridlines, value ticks and the baseline. `suffix` follows each tick.
fn axes(f: &Frame, step: i64, suffix: &str) -> Vec<Mark> {
    let mut out = Vec::new();
    let mut v = 0;
    while v <= f.top {
        let y = f.y(v);
        if v > 0 {
            out.push(mark(
                "line",
                "grid",
                vec![("x1", LEFT), ("x2", WIDTH - RIGHT), ("y1", y), ("y2", y)],
            ));
        }
        out.push(text(
            "tick",
            LEFT - 8,
            y + 4,
            "end",
            format!("{}{suffix}", compact(v)),
        ));
        v += step;
    }
    let base = f.y(0);
    out.push(mark(
        "line",
        "axis",
        vec![
            ("x1", LEFT),
            ("x2", WIDTH - RIGHT),
            ("y1", base),
            ("y2", base),
        ],
    ));
    out
}

fn x_labels(f: &Frame, labels: &[String]) -> Vec<Mark> {
    labels
        .iter()
        .enumerate()
        .filter(|(i, _)| f.shows_label(*i))
        .map(|(i, l)| text("tick", f.cx(i), HEIGHT - BOTTOM + 22, "middle", l.clone()))
        .collect()
}

/// A hit area over a whole band, with a guide line shown on hover.
fn hit(f: &Frame, i: usize, tip: usize, guide: bool) -> Mark {
    let x = f.start + f.band * i as i64;
    let mut area = mark(
        "rect",
        "hit-area",
        vec![("x", x), ("y", TOP), ("width", f.band), ("height", PLOT_H)],
    );
    area.attrs.push(("tabindex", "0".to_string()));
    area.tip = Some(tip);
    let mut g = mark("g", "hit", vec![]);
    if guide {
        let cx = f.cx(i);
        g.children.push(mark(
            "line",
            "guide",
            vec![("x1", cx), ("x2", cx), ("y1", TOP), ("y2", TOP + PLOT_H)],
        ));
    }
    g.children.push(area);
    g
}

/// A column segment from `y_low` up to `y_high`; rounded on top if `cap`.
fn column(class: &str, x: i64, w: i64, y_low: i64, y_high: i64, cap: bool) -> Mark {
    if !cap {
        return mark(
            "rect",
            class,
            vec![
                ("x", x),
                ("y", y_high),
                ("width", w),
                ("height", y_low - y_high),
            ],
        );
    }
    let r = RADIUS.min(y_low - y_high).min(w / 2);
    path(
        class,
        format!(
            "M{x} {y_low}V{}Q{x} {y_high} {} {y_high}H{}Q{} {y_high} {} {}V{y_low}Z",
            y_high + r,
            x + r,
            x + w - r,
            x + w,
            x + w,
            y_high + r
        ),
    )
}

/// One stacked column per period. `stacks[i]` lists (class, value) from the
/// baseline up; `labels_above[i]` is the text over the column, if any.
pub struct Stacked<'a> {
    pub key: &'a str,
    pub title: String,
    pub note: String,
    pub aria: String,
    pub legend: Vec<Legend>,
    pub x_labels: Vec<String>,
    pub stacks: Vec<Vec<(&'static str, i64)>>,
    pub partial: Vec<bool>,
    pub labels_above: Vec<Option<String>>,
    pub tips: Vec<Vec<String>>,
}

pub fn stacked(s: Stacked) -> Chart {
    let totals: Vec<i64> = s
        .stacks
        .iter()
        .map(|st| st.iter().map(|(_, v)| v).sum())
        .collect();
    let (step, top) = scale(totals.iter().copied().max().unwrap_or(0));
    let f = Frame::new(s.stacks.len(), top);
    let mut marks = axes(&f, step, "");
    let w = f.bar_w();
    let n = s.stacks.len();
    for (i, stack) in s.stacks.iter().enumerate() {
        let x = f.cx(i) - w / 2;
        let drawn: Vec<&(&str, i64)> = stack.iter().filter(|(_, v)| *v > 0).collect();
        let mut sum = 0;
        for (j, (class, v)) in drawn.iter().enumerate() {
            let y_low = f.y(sum) - if j > 0 { GAP } else { 0 };
            sum += v;
            let y_high = f.y(sum);
            if y_low <= y_high {
                continue;
            }
            let class = if s.partial[i] {
                format!("{class} partial")
            } else {
                class.to_string()
            };
            marks.push(column(&class, x, w, y_low, y_high, j + 1 == drawn.len()));
        }
        let labelled = n <= LABEL_ALL_UP_TO || i + LABEL_LAST >= n;
        if let (true, Some(label)) = (labelled, &s.labels_above[i]) {
            marks.push(text(
                "bar-label",
                f.cx(i),
                f.y(totals[i]) - 8,
                "middle",
                label.clone(),
            ));
        }
    }
    marks.extend(x_labels(&f, &s.x_labels));
    for i in 0..n {
        marks.push(hit(&f, i, i, false));
    }
    Chart {
        key: s.key.to_string(),
        title: s.title,
        note: s.note,
        width: WIDTH,
        height: HEIGHT,
        legend: s.legend,
        marks,
        tips: s.tips,
        aria: s.aria,
    }
}

/// One series of a line chart; `None` leaves a gap.
pub struct Series {
    pub class: String,
    pub label: String,
    pub values: Vec<Option<i64>>,
    /// A light wash under the line (one series only).
    pub area: bool,
}

pub struct Lines<'a> {
    pub key: &'a str,
    pub title: String,
    pub note: String,
    pub aria: String,
    pub x_labels: Vec<String>,
    pub series: Vec<Series>,
    pub tips: Vec<Vec<String>>,
    /// A fixed axis top and its step, and a suffix for the ticks: used for
    /// percentages.
    pub fixed: Option<(i64, i64, &'static str)>,
    /// A labelled reference line: (value, label).
    pub target: Option<(i64, String)>,
    /// Dots below the target get the class "below".
    pub dots_below_target: bool,
}

pub fn lines(l: Lines) -> Chart {
    let n = l.x_labels.len();
    let max = l
        .series
        .iter()
        .flat_map(|s| s.values.iter().flatten().copied())
        .max()
        .unwrap_or(0);
    let (step, top, suffix) = match l.fixed {
        Some((top, step, suffix)) => (step, top, suffix),
        None => {
            let (step, top) = scale(max);
            (step, top, "")
        }
    };
    let f = Frame::new(n, top);
    let mut marks = axes(&f, step, suffix);
    if let Some((v, label)) = &l.target {
        let y = f.y(*v);
        marks.push(mark(
            "line",
            "target",
            vec![("x1", LEFT), ("x2", WIDTH - RIGHT), ("y1", y), ("y2", y)],
        ));
        marks.push(text(
            "target-label",
            WIDTH - RIGHT,
            y - 6,
            "end",
            label.clone(),
        ));
    }
    let dots_everywhere = n <= MAX_X_LABELS as usize;
    for s in &l.series {
        let points: Vec<Option<(i64, i64)>> = s
            .values
            .iter()
            .enumerate()
            .map(|(i, v)| v.map(|v| (f.cx(i), f.y(v))))
            .collect();
        let mut d = String::new();
        let mut pen_down = false;
        for p in &points {
            match p {
                Some((x, y)) => {
                    d.push_str(&format!("{}{x} {y}", if pen_down { "L" } else { "M" }));
                    pen_down = true;
                }
                None => pen_down = false,
            }
        }
        if s.area {
            // Each unbroken run gets its own wash down to the baseline.
            let base = f.y(0);
            for run in points.split(|p| p.is_none()).filter(|r| r.len() > 1) {
                let pts: Vec<(i64, i64)> = run.iter().flatten().copied().collect();
                let mut a = format!("M{} {base}", pts[0].0);
                for (x, y) in &pts {
                    a.push_str(&format!("L{x} {y}"));
                }
                a.push_str(&format!("L{} {base}Z", pts[pts.len() - 1].0));
                marks.push(path(&format!("{} area", s.class), a));
            }
        }
        if !d.is_empty() {
            marks.push(path(&format!("{} line", s.class), d));
        }
        let last = points.iter().rposition(|p| p.is_some());
        for (i, p) in points.iter().enumerate() {
            if let Some((x, y)) = p {
                if dots_everywhere || Some(i) == last {
                    let below = l.dots_below_target
                        && matches!((&l.target, s.values[i]), (Some((t, _)), Some(v)) if v < *t);
                    let class = format!("{} dot{}", s.class, if below { " below" } else { "" });
                    marks.push(mark(
                        "circle",
                        &class,
                        vec![("cx", *x), ("cy", *y), ("r", RADIUS)],
                    ));
                }
            }
        }
    }
    marks.extend(x_labels(&f, &l.x_labels));
    for i in 0..n {
        marks.push(hit(&f, i, i, true));
    }
    let legend = if l.series.len() > 1 {
        l.series
            .iter()
            .map(|s| Legend {
                class: s.class.clone(),
                label: s.label.clone(),
            })
            .collect()
    } else {
        Vec::new()
    };
    Chart {
        key: l.key.to_string(),
        title: l.title,
        note: l.note,
        width: WIDTH,
        height: HEIGHT,
        legend,
        marks,
        tips: l.tips,
        aria: l.aria,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attr(m: &Mark, k: &str) -> i64 {
        m.attrs
            .iter()
            .find(|(a, _)| *a == k)
            .unwrap()
            .1
            .parse()
            .unwrap()
    }

    #[test]
    fn scales_are_round() {
        assert_eq!(scale(0), (1, 1));
        assert_eq!(scale(4), (1, 4));
        assert_eq!(scale(7), (2, 8));
        assert_eq!(scale(9), (5, 10));
        assert_eq!(scale(37), (10, 40));
        assert_eq!(scale(41), (20, 60));
        assert_eq!(scale(1234), (500, 1500));
    }

    #[test]
    fn numbers_are_written_compactly() {
        assert_eq!(compact(950), "950");
        assert_eq!(compact(1000), "1K");
        assert_eq!(compact(1500), "1.5K");
        assert_eq!(compact(12500), "12K");
        assert_eq!(thousands(1284), "1,284");
        assert_eq!(thousands(1234567), "1,234,567");
        assert_eq!(thousands(12), "12");
        assert_eq!(thousands(-1000), "\u{2212}1,000");
    }

    fn two_columns() -> Chart {
        stacked(Stacked {
            key: "k",
            title: "T".into(),
            note: "N".into(),
            aria: "A".into(),
            legend: vec![],
            x_labels: vec!["a".into(), "b".into()],
            stacks: vec![
                vec![("s-done", 2), ("s-pending", 2)],
                vec![("s-done", 4), ("s-pending", 0)],
            ],
            partial: vec![false, true],
            labels_above: vec![None, Some("+0%".into())],
            tips: vec![vec!["a".into()], vec!["b".into()]],
        })
    }

    #[test]
    fn stacked_segments_meet_the_scale_with_a_gap() {
        let c = two_columns();
        // Top is 4 over a plot 270 high: 0 at y=300, 2 at y=165, 4 at y=30.
        let segs: Vec<&Mark> = c
            .marks
            .iter()
            .filter(|m| m.class.starts_with("s-"))
            .collect();
        assert_eq!(segs.len(), 3);
        assert_eq!(segs[0].tag, "rect");
        assert_eq!(attr(segs[0], "y"), 165);
        assert_eq!(attr(segs[0], "height"), 135);
        // The top segment is a path with a rounded top, 2 above the lower one.
        assert_eq!(segs[1].tag, "path");
        assert!(
            segs[1].attrs[0].1.starts_with("M") && segs[1].attrs[0].1.contains("163V34Q"),
            "{:?}",
            segs[1].attrs
        );
        assert_eq!(segs[2].class, "s-done partial");
        let labels: Vec<&Mark> = c.marks.iter().filter(|m| m.class == "bar-label").collect();
        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0].text, "+0%");
        assert_eq!(attr(labels[0], "y"), 22);
    }

    #[test]
    fn columns_are_at_most_24_wide_and_centred_in_their_band() {
        let c = two_columns();
        let first = c.marks.iter().find(|m| m.class == "s-done").unwrap();
        assert_eq!(attr(first, "width"), 24);
        // Two bands of 442 from x=56: the first centre is 277.
        assert_eq!(attr(first, "x"), 277 - 12);
    }

    #[test]
    fn each_band_has_one_hit_area_with_its_tip() {
        let c = two_columns();
        let hits: Vec<&Mark> = c.marks.iter().filter(|m| m.class == "hit").collect();
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[1].children[0].tip, Some(1));
    }

    #[test]
    fn many_x_labels_are_thinned_from_the_last() {
        let f = Frame::new(36, 10);
        let shown: Vec<usize> = (0..36).filter(|i| f.shows_label(*i)).collect();
        assert_eq!(shown.len(), 12);
        assert_eq!(*shown.last().unwrap(), 35);
        assert_eq!(shown[0], 2);
    }

    #[test]
    fn axes_labels_and_hit_areas_sit_where_the_frame_puts_them() {
        let c = two_columns();
        // Ticks 0 to 4, step 1, each 4 below its line: y = 300 - 67v + 4
        // (270 / 4 = 67.5, whole-number division per value).
        let ticks: Vec<(String, i64, i64)> = c
            .marks
            .iter()
            .filter(|m| m.class == "tick" && attr(m, "x") == 48)
            .map(|m| (m.text.clone(), attr(m, "x"), attr(m, "y")))
            .collect();
        assert_eq!(
            ticks,
            vec![
                ("0".into(), 48, 304),
                ("1".into(), 48, 237),
                ("2".into(), 48, 169),
                ("3".into(), 48, 102),
                ("4".into(), 48, 34)
            ]
        );
        // Four gridlines (none at 0) and a baseline at 300.
        assert_eq!(c.marks.iter().filter(|m| m.class == "grid").count(), 4);
        let base = c.marks.iter().find(|m| m.class == "axis").unwrap();
        assert_eq!(
            (attr(base, "x1"), attr(base, "x2"), attr(base, "y1")),
            (56, 940, 300)
        );
        // X labels under each band centre: 277 and 719, at y = 322.
        let xs: Vec<(i64, i64)> = c
            .marks
            .iter()
            .filter(|m| m.class == "tick" && (m.text == "a" || m.text == "b"))
            .map(|m| (attr(m, "x"), attr(m, "y")))
            .collect();
        assert_eq!(xs, vec![(277, 322), (719, 322)]);
        // The second hit area covers the second band, top to baseline.
        let hit = &c
            .marks
            .iter()
            .filter(|m| m.class == "hit")
            .nth(1)
            .unwrap()
            .children[0];
        assert_eq!(
            (
                attr(hit, "x"),
                attr(hit, "y"),
                attr(hit, "width"),
                attr(hit, "height")
            ),
            (498, 30, 442, 270)
        );
    }

    #[test]
    fn a_capped_column_has_a_four_unit_rounded_top() {
        let m = column("s", 265, 24, 163, 30, true);
        assert_eq!(
            m.attrs[0].1,
            "M265 163V34Q265 30 269 30H285Q289 30 289 34V163Z"
        );
        // A segment shorter than the radius rounds by its own height.
        let m = column("s", 0, 24, 12, 10, true);
        assert_eq!(m.attrs[0].1, "M0 12V12Q0 10 2 10H22Q24 10 24 12V12Z");
        let m = column("s", 0, 24, 12, 10, false);
        assert_eq!((attr(&m, "y"), attr(&m, "height")), (10, 2));
    }

    #[test]
    fn a_line_and_its_wash_follow_the_points() {
        // Three bands of 294 from x = 57: centres 204, 498, 792. Top 10 over
        // 270: y(5) = 165, y(10) = 30, y(0) = 300.
        let c = lines(Lines {
            key: "o",
            title: "T".into(),
            note: "N".into(),
            aria: "A".into(),
            x_labels: vec!["a".into(), "b".into(), "c".into()],
            series: vec![Series {
                class: "s-open".into(),
                label: "Open".into(),
                values: vec![Some(5), Some(10), Some(0)],
                area: true,
            }],
            tips: vec![vec![], vec![], vec![]],
            fixed: None,
            target: None,
            dots_below_target: false,
        });
        let line = c.marks.iter().find(|m| m.class == "s-open line").unwrap();
        assert_eq!(line.attrs[0].1, "M204 165L498 30L792 300");
        let area = c.marks.iter().find(|m| m.class == "s-open area").unwrap();
        assert_eq!(area.attrs[0].1, "M204 300L204 165L498 30L792 300L792 300Z");
        // A guide line in each hit group, at the band centre.
        let guide = &c
            .marks
            .iter()
            .filter(|m| m.class == "hit")
            .nth(1)
            .unwrap()
            .children[0];
        assert_eq!(
            (guide.class.as_str(), attr(guide, "x1"), attr(guide, "y2")),
            ("guide", 498, 300)
        );
    }

    #[test]
    fn lines_break_at_gaps_and_mark_dots_below_the_target() {
        let c = lines(Lines {
            key: "r",
            title: "T".into(),
            note: "N".into(),
            aria: "A".into(),
            x_labels: vec!["a".into(), "b".into(), "c".into()],
            series: vec![Series {
                class: "s-rate".into(),
                label: "Rate".into(),
                values: vec![Some(90), None, Some(50)],
                area: false,
            }],
            tips: vec![vec![], vec![], vec![]],
            fixed: Some((100, 25, "%")),
            target: Some((80, "Target 80%".into())),
            dots_below_target: true,
        });
        let line = c.marks.iter().find(|m| m.class == "s-rate line").unwrap();
        assert_eq!(line.attrs[0].1.matches('M').count(), 2);
        let dots: Vec<&Mark> = c.marks.iter().filter(|m| m.tag == "circle").collect();
        assert_eq!(dots.len(), 2);
        assert_eq!(dots[0].class, "s-rate dot");
        assert_eq!(dots[1].class, "s-rate dot below");
        let target = c.marks.iter().find(|m| m.class == "target").unwrap();
        // 80 of 100 over 270: y = 300 - 216 = 84.
        assert_eq!(attr(target, "y1"), 84);
        assert!(c
            .marks
            .iter()
            .any(|m| m.class == "tick" && m.text == "100%"));
        assert!(c.legend.is_empty());
    }
}
