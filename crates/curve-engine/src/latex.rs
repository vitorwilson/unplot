//! Render a fitted spline as LaTeX, deterministically from the engine's own
//! coefficients (no CAS). The frontend displays this with KaTeX; keeping the
//! string generation here makes it headless-testable and identical everywhere.
//!
//! The polynomial and number formatting live in `coeffs`, shared with the Desmos
//! and Wolfram export forms (`export`); this module owns only the LaTeX-specific
//! wrapping — the `cases` block, the summary line, and the interval conditions.

use crate::coeffs::{fmt_num, poly};
use crate::spline::Spline;

/// One-line plain-text summary, e.g. `"12-segment spline over [-3.2, 4.1]"`.
/// Shown collapsed by default so a hundred-segment curve stays legible.
pub fn summary(spline: &Spline) -> String {
    let (a, b) = spline.domain();
    let n = spline.segments().len();
    let noun = if n == 1 { "segment" } else { "segments" };
    format!("{n}-{noun} spline over [{}, {}]", fmt_num(a), fmt_num(b))
}

/// The exact function as a LaTeX `cases` block: one cubic per segment over its
/// half-open interval (the last interval is closed at both ends).
pub fn piecewise(spline: &Spline) -> String {
    let segments = spline.segments();
    let last = segments.len() - 1;
    let rows: Vec<String> = segments
        .iter()
        .enumerate()
        .map(|(i, seg)| {
            format!(
                "{} & {}",
                poly(seg, true),
                interval(seg.x_start, seg.x_end, i == last)
            )
        })
        .collect();
    format!(
        "f(x) = \\begin{{cases}} {} \\end{{cases}}",
        rows.join(" \\\\ ")
    )
}

/// The interval condition, e.g. `0 \le x < 1` (or `\le` at both ends for the
/// final segment).
fn interval(a: f64, b: f64, closed: bool) -> String {
    let upper = if closed { "\\le" } else { "<" };
    format!("{} \\le x {} {}", fmt_num(a), upper, fmt_num(b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Curve, Knot};

    fn fit(knots: Vec<Knot>) -> Spline {
        Curve::new(knots).unwrap().fit()
    }

    #[test]
    fn summary_counts_segments_and_reports_domain() {
        let spline = fit(vec![
            Knot::new(0.0, 0.0),
            Knot::new(1.0, 1.0),
            Knot::new(2.0, 0.0),
        ]);
        assert_eq!(summary(&spline), "2-segments spline over [0, 2]");
    }

    #[test]
    fn summary_is_singular_for_one_segment() {
        let spline = fit(vec![Knot::new(0.0, 0.0), Knot::new(2.0, 4.0)]);
        assert_eq!(summary(&spline), "1-segment spline over [0, 2]");
    }

    #[test]
    fn a_line_through_the_origin_is_its_slope_times_x() {
        // f(x) = 2x on [0, 2]: constant/quadratic/cubic terms vanish.
        let latex = piecewise(&fit(vec![Knot::new(0.0, 0.0), Knot::new(2.0, 4.0)]));
        assert!(latex.contains("2x"), "latex was: {latex}");
        assert!(latex.contains("\\begin{cases}"), "latex was: {latex}");
        assert!(latex.contains("0 \\le x \\le 2"), "latex was: {latex}");
    }

    #[test]
    fn an_interior_interval_is_half_open() {
        // A two-segment curve: the first interval is closed-open, the last closed.
        let latex = piecewise(&fit(vec![
            Knot::new(0.0, 0.0),
            Knot::new(1.0, 1.0),
            Knot::new(2.0, 0.0),
        ]));
        assert!(latex.contains("0 \\le x < 1"), "latex was: {latex}");
        assert!(latex.contains("1 \\le x \\le 2"), "latex was: {latex}");
    }

    #[test]
    fn a_flat_line_is_a_constant() {
        let latex = piecewise(&fit(vec![Knot::new(0.0, 3.0), Knot::new(2.0, 3.0)]));
        assert!(latex.contains("3 &"), "latex was: {latex}");
    }
}
