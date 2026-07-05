//! Render a fitted spline as LaTeX, deterministically from the engine's own
//! coefficients (no CAS). The frontend displays this with KaTeX; keeping the
//! string generation here makes it headless-testable and identical everywhere.

use crate::spline::{Segment, Spline};

// Coefficients and coordinates below this magnitude are shown as zero, so tiny
// floating-point residue doesn't clutter the output. Display only — the curve
// itself is unchanged.
const DISPLAY_EPS: f64 = 5e-5;

/// One-line plain-text summary, e.g. `"12-segment spline over [-3.2, 4.1]"`.
/// Shown collapsed by default so a hundred-segment curve stays legible.
pub fn summary(spline: &Spline) -> String {
    let (a, b) = spline.domain();
    let n = spline.segments().len();
    let noun = if n == 1 { "segment" } else { "segments" };
    format!("{n}-{noun} spline over [{}, {}]", fmt(a), fmt(b))
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
                segment_poly(seg),
                interval(seg.x_start, seg.x_end, i == last)
            )
        })
        .collect();
    format!(
        "f(x) = \\begin{{cases}} {} \\end{{cases}}",
        rows.join(" \\\\ ")
    )
}

/// The cubic `a + b·(x−x₀) + c·(x−x₀)² + d·(x−x₀)³` for one segment, dropping
/// near-zero terms and folding signs.
fn segment_poly(seg: &Segment) -> String {
    let x0 = seg.x_start;
    let mut out = String::new();
    for (power, &coeff) in seg.coeffs.iter().enumerate() {
        if coeff.abs() < DISPLAY_EPS {
            continue;
        }
        let term = fmt_term(coeff.abs(), &shift(x0, power as u32));
        if out.is_empty() {
            out.push_str(if coeff < 0.0 { "-" } else { "" });
        } else {
            out.push_str(if coeff < 0.0 { " - " } else { " + " });
        }
        out.push_str(&term);
    }
    if out.is_empty() {
        "0".to_string()
    } else {
        out
    }
}

/// A coefficient's magnitude times its `(x−x₀)ᵏ` factor, omitting a unit
/// coefficient (`1·(x−1)` → `(x−1)`) and an empty factor (the constant term).
fn fmt_term(abs_coeff: f64, factor: &str) -> String {
    if factor.is_empty() {
        return fmt(abs_coeff);
    }
    if (abs_coeff - 1.0).abs() < DISPLAY_EPS {
        return factor.to_string();
    }
    format!("{}{}", fmt(abs_coeff), factor)
}

/// The `(x−x₀)ᵏ` factor for a power, simplifying `x₀ = 0` to `x` and power 1 to
/// no exponent.
fn shift(x0: f64, power: u32) -> String {
    if power == 0 {
        return String::new();
    }
    let base = if x0.abs() < DISPLAY_EPS {
        "x".to_string()
    } else if x0 > 0.0 {
        format!("(x - {})", fmt(x0))
    } else {
        format!("(x + {})", fmt(-x0))
    };
    if power == 1 {
        base
    } else {
        format!("{base}^{{{power}}}")
    }
}

/// The interval condition, e.g. `0 \le x < 1` (or `\le` at both ends for the
/// final segment).
fn interval(a: f64, b: f64, closed: bool) -> String {
    let upper = if closed { "\\le" } else { "<" };
    format!("{} \\le x {} {}", fmt(a), upper, fmt(b))
}

/// Format a number for display: at most four decimals, trailing zeros trimmed,
/// negative-zero normalized to `0`.
fn fmt(value: f64) -> String {
    let v = if value.abs() < DISPLAY_EPS {
        0.0
    } else {
        value
    };
    let s = format!("{v:.4}");
    let trimmed = s.trim_end_matches('0').trim_end_matches('.');
    if trimmed.is_empty() || trimmed == "-0" {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
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
    fn a_shifted_segment_uses_x_minus_x0() {
        // A line from (1,0) to (2,1) has slope 1 about x0 = 1: the factor shows.
        let latex = piecewise(&fit(vec![Knot::new(1.0, 0.0), Knot::new(2.0, 1.0)]));
        assert!(latex.contains("(x - 1)"), "latex was: {latex}");
    }

    #[test]
    fn a_flat_line_is_a_constant() {
        let latex = piecewise(&fit(vec![Knot::new(0.0, 3.0), Knot::new(2.0, 3.0)]));
        assert!(latex.contains("3 &"), "latex was: {latex}");
    }

    #[test]
    fn negative_x0_reads_as_x_plus() {
        let latex = piecewise(&fit(vec![Knot::new(-2.0, 0.0), Knot::new(-1.0, 1.0)]));
        assert!(latex.contains("(x + 2)"), "latex was: {latex}");
    }
}
