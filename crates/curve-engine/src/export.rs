//! Serialize a fitted spline into the piecewise syntax other math tools ingest:
//! Desmos (`\left\{cond: expr, …\right\}`) and Wolfram (`Piecewise[{{expr,
//! cond}, …}]`). Both differ from the KaTeX `cases` block in `latex`, which
//! neither tool parses as-is — that gap is the whole reason this module exists
//! (docs/PLAN.md, Phase 4.5).
//!
//! Both targets pick the *first* matching branch when intervals touch at a knot,
//! so we emit closed intervals `[x_start, x_end]` on every piece: the spline is
//! continuous, so adjacent pieces agree at the shared knot and precedence is
//! irrelevant to the value. That's simpler and safer than mixing `≤`/`<`, which
//! Desmos does not reliably accept in a compound inequality.

use crate::coeffs::{fmt_num, poly};
use crate::spline::{Segment, Spline};

/// The spline as Desmos-pasteable piecewise LaTeX. Pasting the string into a
/// fresh Desmos expression plots `y = f(x)` over the drawn domain.
///
/// # Example
/// ```
/// use curve_engine::{Curve, Knot};
/// let spline = Curve::new(vec![Knot::new(0.0, 0.0), Knot::new(2.0, 4.0)])
///     .unwrap()
///     .fit();
/// assert_eq!(
///     curve_engine::export::desmos(&spline),
///     "\\left\\{0 \\le x \\le 2: 2x\\right\\}"
/// );
/// ```
pub fn desmos(spline: &Spline) -> String {
    let pieces: Vec<String> = spline
        .segments()
        .iter()
        .map(|seg| format!("{}: {}", desmos_condition(seg), poly(seg, true)))
        .collect();
    // `\left\{ … \right\}` — the doubled braces are format!'s escapes for one
    // literal `{` / `}` around the LaTeX-escaped Desmos delimiters.
    format!("\\left\\{{{}\\right\\}}", pieces.join(", "))
}

/// The spline as a Wolfram Language `Piecewise[…]`. Pasting into Wolfram|Alpha
/// or Mathematica reproduces the drawn curve (0 outside the domain, Wolfram's
/// default for an unmatched piece).
///
/// # Example
/// ```
/// use curve_engine::{Curve, Knot};
/// let spline = Curve::new(vec![Knot::new(0.0, 0.0), Knot::new(2.0, 4.0)])
///     .unwrap()
///     .fit();
/// assert_eq!(
///     curve_engine::export::wolfram(&spline),
///     "Piecewise[{{2x, 0 <= x <= 2}}]"
/// );
/// ```
pub fn wolfram(spline: &Spline) -> String {
    let pieces: Vec<String> = spline
        .segments()
        .iter()
        .map(|seg| format!("{{{}, {}}}", poly(seg, false), wolfram_condition(seg)))
        .collect();
    format!("Piecewise[{{{}}}]", pieces.join(", "))
}

/// A closed interval condition in Desmos LaTeX, e.g. `0 \le x \le 1`.
fn desmos_condition(seg: &Segment) -> String {
    format!(
        "{} \\le x \\le {}",
        fmt_num(seg.x_start),
        fmt_num(seg.x_end)
    )
}

/// A closed interval condition in Wolfram Language, e.g. `0 <= x <= 1`.
fn wolfram_condition(seg: &Segment) -> String {
    format!("{} <= x <= {}", fmt_num(seg.x_start), fmt_num(seg.x_end))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Curve, Knot};

    fn fit(knots: Vec<Knot>) -> Spline {
        Curve::new(knots).unwrap().fit()
    }

    #[test]
    fn desmos_wraps_one_piece_in_a_bracket_list() {
        let out = desmos(&fit(vec![Knot::new(0.0, 0.0), Knot::new(2.0, 4.0)]));
        assert_eq!(out, "\\left\\{0 \\le x \\le 2: 2x\\right\\}");
    }

    #[test]
    fn wolfram_wraps_one_piece_in_a_piecewise_call() {
        let out = wolfram(&fit(vec![Knot::new(0.0, 0.0), Knot::new(2.0, 4.0)]));
        assert_eq!(out, "Piecewise[{{2x, 0 <= x <= 2}}]");
    }

    #[test]
    fn desmos_uses_closed_intervals_and_comma_joins_pieces() {
        // A two-segment curve: both pieces closed on both ends, comma-separated.
        let out = desmos(&fit(vec![
            Knot::new(0.0, 0.0),
            Knot::new(1.0, 1.0),
            Knot::new(2.0, 0.0),
        ]));
        assert!(out.starts_with("\\left\\{"), "was: {out}");
        assert!(out.ends_with("\\right\\}"), "was: {out}");
        assert!(out.contains("0 \\le x \\le 1: "), "was: {out}");
        assert!(out.contains("1 \\le x \\le 2: "), "was: {out}");
        assert_eq!(out.matches(", ").count(), 1, "one comma between two pieces");
    }

    #[test]
    fn wolfram_lists_each_piece_as_expr_then_condition() {
        let out = wolfram(&fit(vec![
            Knot::new(0.0, 0.0),
            Knot::new(1.0, 1.0),
            Knot::new(2.0, 0.0),
        ]));
        assert!(out.starts_with("Piecewise[{{"), "was: {out}");
        assert!(out.ends_with("}}]"), "was: {out}");
        assert!(out.contains(", 0 <= x <= 1}"), "was: {out}");
        assert!(out.contains(", 1 <= x <= 2}"), "was: {out}");
    }

    #[test]
    fn wolfram_emits_brace_free_exponents_for_a_cubic() {
        // A smoothstep (flat user tangents) has a real cubic term.
        let out = wolfram(&fit(vec![
            Knot::with_tangent(0.0, 0.0, 0.0),
            Knot::with_tangent(1.0, 1.0, 0.0),
        ]));
        assert!(out.contains("^3"), "was: {out}");
        assert!(
            !out.contains("^{"),
            "Wolfram exponents take no braces: {out}"
        );
    }

    #[test]
    fn both_targets_shift_a_segment_by_its_left_endpoint() {
        let knots = vec![Knot::new(1.0, 0.0), Knot::new(2.0, 1.0)];
        assert!(desmos(&fit(knots.clone())).contains("(x - 1)"));
        assert!(wolfram(&fit(knots)).contains("(x - 1)"));
    }
}
