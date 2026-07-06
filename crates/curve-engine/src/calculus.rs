//! Analytic calculus on a fitted curve, per segment, with no CAS. Each piece is
//! a polynomial in power basis about its left endpoint, so differentiation and
//! integration are term-by-term (Phase 5, docs/PLAN.md). The result is itself a
//! [`Spline`], so it evaluates, renders as LaTeX/Desmos/Wolfram, and can be
//! differentiated or integrated again (chaining).

use crate::spline::{Segment, Spline};

/// The exact derivative `f'`. Each cubic piece becomes its quadratic derivative
/// about the same left endpoint; the result is continuous (the source is C¹, so
/// the one-sided slopes already agree at every knot) but only C⁰ — it has
/// corners at the knots, which the UI should surface rather than hide.
///
/// # Example
/// ```
/// use curve_engine::{calculus, Curve, Knot};
/// // f(x) = 2x  ⇒  f'(x) = 2 everywhere.
/// let line = Curve::new(vec![Knot::new(0.0, 0.0), Knot::new(2.0, 4.0)])
///     .unwrap()
///     .fit();
/// assert!((calculus::differentiate(&line).eval(1.0) - 2.0).abs() < 1e-9);
/// ```
pub fn differentiate(spline: &Spline) -> Spline {
    Spline::from_pieces(spline.segments().iter().map(diff_piece).collect())
}

/// The exact antiderivative `F` with `F(x_first) = 0`. Each piece is integrated
/// term-by-term into a polynomial one degree higher, and its constant is the
/// definite integral accumulated up to that piece's left endpoint — so `F` is
/// continuous across every join (and C², since `F' = f` is C¹).
///
/// # Example
/// ```
/// use curve_engine::{calculus, Curve, Knot};
/// // ∫ 2x dx from 0 = x²  ⇒  F(2) = 4.
/// let line = Curve::new(vec![Knot::new(0.0, 0.0), Knot::new(2.0, 4.0)])
///     .unwrap()
///     .fit();
/// assert!((calculus::integrate(&line).eval(2.0) - 4.0).abs() < 1e-9);
/// ```
pub fn integrate(spline: &Spline) -> Spline {
    let mut accumulated = 0.0;
    let mut pieces = Vec::with_capacity(spline.segments().len());
    for seg in spline.segments() {
        pieces.push(integ_piece(seg, accumulated));
        accumulated += definite_integral(seg);
    }
    Spline::from_pieces(pieces)
}

/// Differentiate one piece: `Σ cₖtᵏ → Σ_{k≥1} k·cₖ·tᵏ⁻¹`. A degree-0 (constant)
/// piece differentiates to the zero polynomial, kept as `[0]` so the piece is
/// never empty.
fn diff_piece(seg: &Segment) -> Segment {
    let coeffs: Vec<f64> = seg
        .coeffs
        .iter()
        .enumerate()
        .skip(1)
        .map(|(power, &c)| power as f64 * c)
        .collect();
    Segment {
        x_start: seg.x_start,
        x_end: seg.x_end,
        coeffs: if coeffs.is_empty() { vec![0.0] } else { coeffs },
    }
}

/// Integrate one piece about its left endpoint, with the given integration
/// constant: `Σ cₖtᵏ → constant + Σ cₖ/(k+1)·tᵏ⁺¹`.
fn integ_piece(seg: &Segment, constant: f64) -> Segment {
    let mut coeffs = Vec::with_capacity(seg.coeffs.len() + 1);
    coeffs.push(constant);
    for (power, &c) in seg.coeffs.iter().enumerate() {
        coeffs.push(c / (power as f64 + 1.0));
    }
    Segment {
        x_start: seg.x_start,
        x_end: seg.x_end,
        coeffs,
    }
}

/// The definite integral of one piece over its own span, `∫₀ʰ Σ cₖtᵏ dt` with
/// `h = x_end − x_start` — the amount `F` rises across the piece.
fn definite_integral(seg: &Segment) -> f64 {
    let h = seg.x_end - seg.x_start;
    seg.coeffs
        .iter()
        .enumerate()
        .map(|(power, &c)| c / (power as f64 + 1.0) * h.powi(power as i32 + 1))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Curve, Knot};

    fn fit(knots: Vec<Knot>) -> Spline {
        Curve::new(knots).unwrap().fit()
    }

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-9,
            "expected {actual} ≈ {expected}"
        );
    }

    /// Trapezoidal area under a spline — an independent check for the integral.
    fn numeric_area(spline: &Spline) -> f64 {
        spline
            .polyline(4001)
            .windows(2)
            .map(|w| 0.5 * (w[0].1 + w[1].1) * (w[1].0 - w[0].0))
            .sum()
    }

    #[test]
    fn derivative_of_a_line_is_its_constant_slope() {
        let d = differentiate(&fit(vec![Knot::new(0.0, 0.0), Knot::new(2.0, 4.0)]));
        assert_close(d.eval(0.0), 2.0);
        assert_close(d.eval(1.3), 2.0);
        assert_close(d.eval(2.0), 2.0);
        assert_eq!(d.domain(), (0.0, 2.0)); // same interval as the source
    }

    #[test]
    fn derivative_of_a_quadratic_piece_is_linear() {
        // y = 1 + 3t² about x0 = 0 over [0, 2]  ⇒  y' = 6t.
        let quad = Spline::from_pieces(vec![Segment {
            x_start: 0.0,
            x_end: 2.0,
            coeffs: vec![1.0, 0.0, 3.0],
        }]);
        let d = differentiate(&quad);
        assert_close(d.eval(0.0), 0.0);
        assert_close(d.eval(1.0), 6.0);
        assert_close(d.eval(2.0), 12.0);
    }

    #[test]
    fn derivative_equals_the_slope_at_every_knot() {
        // The derivative curve, sampled at each knot, must equal the spline's own
        // reported slope there — which also proves it is continuous at the joins.
        let knots = vec![
            Knot::new(0.0, 0.0),
            Knot::new(1.0, 3.0),
            Knot::new(2.5, -2.0),
            Knot::new(4.0, 1.0),
        ];
        let s = fit(knots.clone());
        let d = differentiate(&s);
        let slopes = s.knot_slopes();
        for (i, knot) in knots.iter().enumerate() {
            assert_close(d.eval(knot.x), slopes[i]);
        }
    }

    #[test]
    fn integral_of_a_line_is_the_quadratic_area() {
        // ∫ 2x dx from 0 = x².
        let f = integrate(&fit(vec![Knot::new(0.0, 0.0), Knot::new(2.0, 4.0)]));
        assert_close(f.eval(0.0), 0.0);
        assert_close(f.eval(1.0), 1.0);
        assert_close(f.eval(2.0), 4.0);
    }

    #[test]
    fn integral_of_a_constant_is_linear() {
        // ∫ 3 dx from 0 = 3x.
        let f = integrate(&fit(vec![Knot::new(0.0, 3.0), Knot::new(2.0, 3.0)]));
        assert_close(f.eval(0.0), 0.0);
        assert_close(f.eval(2.0), 6.0);
    }

    #[test]
    fn integral_starts_at_zero_and_totals_the_area() {
        let s = fit(vec![
            Knot::new(-1.0, 2.0),
            Knot::new(0.5, -1.0),
            Knot::new(2.0, 3.0),
            Knot::new(3.5, 0.0),
        ]);
        let f = integrate(&s);
        let (a, b) = s.domain();
        assert_close(f.eval(a), 0.0);
        assert!(
            (f.eval(b) - numeric_area(&s)).abs() < 1e-3,
            "analytic {} vs numeric {}",
            f.eval(b),
            numeric_area(&s)
        );
    }

    #[test]
    fn integral_is_continuous_across_joins() {
        let s = fit(vec![
            Knot::new(0.0, 1.0),
            Knot::new(1.0, -2.0),
            Knot::new(2.0, 4.0),
        ]);
        let f = integrate(&s);
        // Approach an interior knot from both sides: the values must converge.
        let eps = 1e-6;
        assert!((f.eval(1.0 - eps) - f.eval(1.0 + eps)).abs() < 1e-4);
    }

    #[test]
    fn differentiating_the_integral_recovers_the_curve() {
        // Fundamental theorem: d/dx ∫ f = f. This is exact per segment and
        // exercises chaining (integrate then differentiate).
        let s = fit(vec![
            Knot::new(0.0, 0.0),
            Knot::new(1.0, 2.0),
            Knot::new(2.0, -1.0),
            Knot::new(3.0, 3.0),
        ]);
        let recovered = differentiate(&integrate(&s));
        for i in 0..=30 {
            let x = 3.0 * i as f64 / 30.0;
            assert_close(recovered.eval(x), s.eval(x));
        }
    }
}
