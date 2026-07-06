//! Shared text primitives for turning a segment's power-basis coefficients into
//! a human-readable polynomial. Reused by every output target — the KaTeX
//! `cases` block (`latex`) and the Desmos / Wolfram export forms (`export`) —
//! so the number rounding and term-folding rules live in exactly one place.

use crate::spline::Segment;

/// Coefficients and coordinates below this magnitude are shown as zero, so tiny
/// floating-point residue doesn't clutter the output. Display only — the curve
/// itself is unchanged.
pub(crate) const DISPLAY_EPS: f64 = 5e-5;

/// Format a number for display: at most four decimals, trailing zeros trimmed,
/// negative-zero normalized to `0`.
///
/// # Example
/// ```ignore
/// assert_eq!(fmt_num(2.0), "2");
/// assert_eq!(fmt_num(-3.5), "-3.5");
/// ```
pub(crate) fn fmt_num(value: f64) -> String {
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

/// The cubic `a + b·(x−x₀) + c·(x−x₀)² + d·(x−x₀)³` for one segment, dropping
/// near-zero terms and folding signs. `exp_braces` picks the exponent syntax:
/// `true` for LaTeX/Desmos (`^{2}`), `false` for Wolfram (`^2`); both accept the
/// same implicit multiplication (`2x`, `3(x - 1)`), so only the exponent differs.
pub(crate) fn poly(seg: &Segment, exp_braces: bool) -> String {
    let x0 = seg.x_start;
    let mut out = String::new();
    for (power, &coeff) in seg.coeffs.iter().enumerate() {
        if coeff.abs() < DISPLAY_EPS {
            continue;
        }
        let term = poly_term(coeff.abs(), &power_factor(x0, power as u32, exp_braces));
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
fn poly_term(abs_coeff: f64, factor: &str) -> String {
    if factor.is_empty() {
        return fmt_num(abs_coeff);
    }
    if (abs_coeff - 1.0).abs() < DISPLAY_EPS {
        return factor.to_string();
    }
    format!("{}{}", fmt_num(abs_coeff), factor)
}

/// The `(x−x₀)ᵏ` factor for a power, simplifying `x₀ = 0` to `x` and power 1 to
/// no exponent. `exp_braces` wraps the exponent in `{}` for LaTeX/Desmos.
fn power_factor(x0: f64, power: u32, exp_braces: bool) -> String {
    if power == 0 {
        return String::new();
    }
    let base = if x0.abs() < DISPLAY_EPS {
        "x".to_string()
    } else if x0 > 0.0 {
        format!("(x - {})", fmt_num(x0))
    } else {
        format!("(x + {})", fmt_num(-x0))
    };
    if power == 1 {
        base
    } else if exp_braces {
        format!("{base}^{{{power}}}")
    } else {
        format!("{base}^{power}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Curve, Knot};

    fn segments(knots: Vec<Knot>) -> Vec<Segment> {
        Curve::new(knots).unwrap().fit().segments().to_vec()
    }

    #[test]
    fn fmt_num_trims_trailing_zeros_and_normalizes_neg_zero() {
        assert_eq!(fmt_num(2.0), "2");
        assert_eq!(fmt_num(-3.5), "-3.5");
        assert_eq!(fmt_num(0.25), "0.25");
        assert_eq!(fmt_num(-0.0), "0");
        assert_eq!(fmt_num(1e-9), "0"); // below DISPLAY_EPS
    }

    #[test]
    fn poly_of_a_line_through_the_origin_is_slope_times_x() {
        // f(x) = 2x on [0, 2]: constant/quadratic/cubic terms vanish.
        let seg = segments(vec![Knot::new(0.0, 0.0), Knot::new(2.0, 4.0)]);
        assert_eq!(poly(&seg[0], true), "2x");
        assert_eq!(poly(&seg[0], false), "2x");
    }

    #[test]
    fn poly_exponent_braces_only_affect_higher_powers() {
        // A user-tangent smoothstep has a genuine cubic term, so ^{3} vs ^3 shows.
        let seg = segments(vec![
            Knot::with_tangent(0.0, 0.0, 0.0),
            Knot::with_tangent(1.0, 1.0, 0.0),
        ]);
        assert!(poly(&seg[0], true).contains("^{3}"));
        assert!(poly(&seg[0], false).contains("^3"));
        assert!(!poly(&seg[0], false).contains("^{"));
    }

    #[test]
    fn poly_shifts_by_x0_and_reads_negative_x0_as_plus() {
        let right = segments(vec![Knot::new(1.0, 0.0), Knot::new(2.0, 1.0)]);
        assert!(poly(&right[0], true).contains("(x - 1)"));
        let left = segments(vec![Knot::new(-2.0, 0.0), Knot::new(-1.0, 1.0)]);
        assert!(poly(&left[0], true).contains("(x + 2)"));
    }

    #[test]
    fn poly_of_a_flat_line_is_its_constant() {
        let seg = segments(vec![Knot::new(0.0, 3.0), Knot::new(2.0, 3.0)]);
        assert_eq!(poly(&seg[0], true), "3");
    }
}
