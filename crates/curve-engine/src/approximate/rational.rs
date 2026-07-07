//! Strategy 3: a low-degree rational `P(x)/Q(x)` (a Padé-style fit) for
//! pole-shaped curves — a drawn hyperbola or resonance peak that no polynomial,
//! `eˣ`/`ln`, or sinusoid can express.
//!
//! The fit is linearized: `f(x)·Q(x) = P(x)` is linear in the unknown numerator
//! and denominator coefficients, so a homogeneous least squares (the smallest
//! right singular vector) recovers them. Homogeneous — rather than pinning
//! `Q(0) = 1` — so a pole may sit anywhere, including the origin (`1/x`). The fit
//! is then gated on *true* error and rejected if `Q` vanishes inside the domain,
//! which would be a spurious asymptote where the drawn curve was finite.

use super::{candidate_of, prettify, Candidate};
use crate::coeffs::DISPLAY_EPS;
use crate::symbolic::Expr;
use nalgebra::DMatrix;

/// Rational shapes read as "pretty" only at low degree.
const MAX_NUM_DEGREE: usize = 2;
const MAX_DEN_DEGREE: usize = 2;
/// A denominator coefficient below this (the fit is unit-norm) is treated as zero
/// when picking the normalization pivot.
const PIVOT_EPS: f64 = 1e-9;
/// Reject a fit whose denominator dips to within this fraction of its own peak
/// anywhere in the domain: a pole inside the drawn range, where the curve was
/// finite — a spurious asymptote, not a real one.
const POLE_RATIO: f64 = 0.02;

/// The simplest low-degree rational `P/Q` that clears the error gate, or `None`.
/// Degrees are tried simplest-first, so the first success is the prettiest.
pub(super) fn rational_candidate(
    fit_x: &[f64],
    fit_y: &[f64],
    err_x: &[f64],
    err_y: &[f64],
    tolerance: f64,
) -> Option<Candidate> {
    degree_pairs().into_iter().find_map(|(num_deg, den_deg)| {
        try_rational(num_deg, den_deg, fit_x, fit_y, err_x, err_y, tolerance)
    })
}

/// `(numerator_degree, denominator_degree)` pairs ordered simplest-first: by total
/// degree, then fewer denominator terms, then fewer numerator terms.
fn degree_pairs() -> Vec<(usize, usize)> {
    let mut pairs: Vec<(usize, usize)> = (0..=MAX_NUM_DEGREE)
        .flat_map(|num| (1..=MAX_DEN_DEGREE).map(move |den| (num, den)))
        .collect();
    pairs.sort_by_key(|&(num, den)| (num + den, den, num));
    pairs
}

/// Fit one `(num_deg, den_deg)` rational, prettify it, and gate on honest error.
fn try_rational(
    num_deg: usize,
    den_deg: usize,
    fit_x: &[f64],
    fit_y: &[f64],
    err_x: &[f64],
    err_y: &[f64],
    tolerance: f64,
) -> Option<Candidate> {
    let num_len = num_deg + 1;
    let cols = num_len + den_deg + 1;
    // A clean fit is only evidence with more points than coefficients (as in the
    // basis strategy): otherwise the rational threads the points and "recognizes"
    // noise.
    if fit_x.len() <= cols {
        return None;
    }
    let raw = homogeneous_fit(num_len, den_deg + 1, fit_x, fit_y)?;
    let (num, den) = normalize(&raw[..num_len], &raw[num_len..]);
    let num: Vec<f64> = num.iter().map(|&c| prettify(c)).collect();
    let den: Vec<f64> = den.iter().map(|&c| prettify(c)).collect();

    if !has_nonconstant(&den) {
        return None; // degenerated to a polynomial — the basis strategy is prettier
    }
    if num.iter().all(|c| c.abs() < DISPLAY_EPS) {
        return None; // numerator rounded to zero — the curve is ≈ 0, not a rational
    }
    if pole_inside(&den, err_x) {
        return None;
    }
    candidate_of(Expr::Rational { num, den }, err_x, err_y, tolerance)
}

/// Solve `P(x) − f(x)·Q(x) = 0` in the least-squares sense over the fit points:
/// the coefficient vector is the smallest right singular vector of the design
/// matrix `[x⁰ … x^m | −f·x⁰ … −f·xⁿ]` (unit-norm, any pole location allowed).
fn homogeneous_fit(
    num_len: usize,
    den_len: usize,
    fit_x: &[f64],
    fit_y: &[f64],
) -> Option<Vec<f64>> {
    let cols = num_len + den_len;
    let design = DMatrix::from_fn(fit_x.len(), cols, |i, j| {
        if j < num_len {
            fit_x[i].powi(j as i32)
        } else {
            -fit_y[i] * fit_x[i].powi((j - num_len) as i32)
        }
    });
    let v_t = design.svd(true, true).v_t?;
    // Singular values are descending, so the last row of Vᵀ is the null-most
    // direction — the coefficients that best drive the residual to zero.
    let coeffs: Vec<f64> = v_t.row(v_t.nrows() - 1).iter().copied().collect();
    coeffs.iter().all(|c| c.is_finite()).then_some(coeffs)
}

/// Scale numerator and denominator to a monic denominator — its highest-order
/// non-zero term becomes `1` — the canonical, pretty form (`1/x`, `1/(2 + x)`,
/// `1/(1 + x²)`). The homogeneous fit's sign is arbitrary, so the scale also
/// flips when the numerator would otherwise lead negative: `1/(1 − x²)` reads
/// cleanly instead of as `−1/(−1 + x²)`.
fn normalize(num: &[f64], den: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let pivot = den
        .iter()
        .rev()
        .copied()
        .find(|c| c.abs() > PIVOT_EPS)
        .unwrap_or(1.0);
    let numerator_leads_negative = num
        .iter()
        .find(|c| c.abs() > PIVOT_EPS)
        .is_some_and(|c| c / pivot < 0.0);
    let scale = if numerator_leads_negative {
        -pivot
    } else {
        pivot
    };
    (
        num.iter().map(|c| c / scale).collect(),
        den.iter().map(|c| c / scale).collect(),
    )
}

/// Whether the denominator has any term above the constant — otherwise `P/Q` is
/// just a polynomial, which the basis strategy renders more simply.
fn has_nonconstant(den: &[f64]) -> bool {
    den.iter().skip(1).any(|c| c.abs() >= DISPLAY_EPS)
}

/// Whether `Q` has a root (a pole) inside the sampled domain: it changes sign, or
/// dips to a small fraction of its own peak. Either means a vertical asymptote
/// where the drawn curve was finite.
fn pole_inside(den: &[f64], xs: &[f64]) -> bool {
    let mut peak = 0.0_f64;
    let mut trough = f64::INFINITY;
    let mut sign = 0_i8;
    for &x in xs {
        let q = horner(den, x);
        peak = peak.max(q.abs());
        trough = trough.min(q.abs());
        let s = q.partial_cmp(&0.0).map_or(0, |o| o as i8);
        if sign == 0 {
            sign = s;
        } else if s != 0 && s != sign {
            return true;
        }
    }
    peak == 0.0 || trough < POLE_RATIO * peak
}

/// Evaluate `Σ cₖ xᵏ` by Horner's method.
fn horner(coeffs: &[f64], x: f64) -> f64 {
    coeffs.iter().rev().fold(0.0, |acc, &c| acc * x + c)
}

#[cfg(test)]
mod tests {
    use super::super::{closed_form_of_knots, ClosedForm};
    use crate::Knot;

    /// Recover the closed form for `n` typed points of `f` over `[a, b]`.
    fn recover(f: impl Fn(f64) -> f64, a: f64, b: f64, n: usize) -> Option<ClosedForm> {
        let knots: Vec<Knot> = (0..n)
            .map(|i| {
                let x = a + (b - a) * i as f64 / (n - 1) as f64;
                Knot::new(x, f(x))
            })
            .collect();
        closed_form_of_knots(&knots)
    }

    #[test]
    fn recovers_one_over_x() {
        let form = recover(|x| 1.0 / x, 0.5, 4.0, 8).expect("1/x is a rational");
        assert_eq!(form.latex, "f(x) \\approx \\frac{1}{x}", "{}", form.latex);
        assert!(form.max_error < 1e-6, "max error {}", form.max_error);
    }

    #[test]
    fn recovers_a_lorentzian() {
        // 1/(1 + x²): even, no pole on the reals — the denominator stays positive.
        let form = recover(|x| 1.0 / (1.0 + x * x), -3.0, 3.0, 13).expect("lorentzian");
        assert_eq!(
            form.latex, "f(x) \\approx \\frac{1}{1 + x^{2}}",
            "{}",
            form.latex
        );
        assert!(form.max_error < 1e-6, "max error {}", form.max_error);
    }

    #[test]
    fn recovers_a_shifted_hyperbola() {
        // 1/(x + 2): a pole at x = −2, safely left of the drawn domain. The monic
        // denominator reads low-to-high, so `x + 2` renders as `2 + x`.
        let form = recover(|x| 1.0 / (x + 2.0), 0.0, 5.0, 9).expect("shifted hyperbola");
        assert_eq!(
            form.latex, "f(x) \\approx \\frac{1}{2 + x}",
            "{}",
            form.latex
        );
        assert!(form.max_error < 1e-6, "max error {}", form.max_error);
    }

    #[test]
    fn renders_a_negative_pole_form_without_double_negatives() {
        // 1/(1 − x²) on (−1, 1): the sign canonicalization must read `1 - x^{2}`,
        // not `-1 + x^{2}`, with a bare `1` numerator.
        let form = recover(|x| 1.0 / (1.0 - x * x), -0.9, 0.9, 11).expect("1/(1 - x^2)");
        assert_eq!(
            form.latex, "f(x) \\approx \\frac{1}{1 - x^{2}}",
            "{}",
            form.latex
        );
    }

    #[test]
    fn a_polynomial_is_not_offered_as_a_rational() {
        // y = x² is a basis term; it must never come back as `x²/1`.
        let form = recover(|x| x * x, 0.0, 3.0, 9).expect("parabola");
        assert!(!form.latex.contains("\\frac"), "{}", form.latex);
    }

    #[test]
    fn stays_silent_for_a_non_rational_squiggle() {
        let form = recover(|x| (4.0 * x).sin() + 0.5 * (9.0 * x).cos(), 0.0, 2.0, 9);
        assert!(form.is_none(), "{:?}", form.map(|f| f.latex));
    }
}
