//! The "prettier function" (Phase 7): when the drawn curve is *basically* a
//! simple function, offer a compact closed form beside the exact piecewise
//! output — never instead of it. It samples the fitted spline and searches for a
//! sparse least-squares fit against a fixed basis dictionary, reporting honest
//! max/RMS error and offering the form only when that error is small enough to be
//! trustworthy. Otherwise it returns `None` and the caller keeps the exact
//! output. FOSS-only, pure-Rust (nalgebra); no CAS. Deterministic.

use crate::coeffs::join_terms;
use crate::spline::Spline;
use nalgebra::{DMatrix, DVector};

/// Points used to fit the coefficients.
const FIT_SAMPLES: usize = 120;
/// A denser grid to measure error honestly between fit points.
const ERROR_SAMPLES: usize = 480;
/// A "pretty" function is a handful of terms, not a dense combination.
const MAX_TERMS: usize = 3;
/// Offer the form only when the largest deviation is within this fraction of the
/// curve's y-range, plus a tiny absolute floor for (near-)flat curves.
const REL_TOLERANCE: f64 = 0.03;
const ABS_TOLERANCE: f64 = 1e-6;
/// A basis value above this over the domain marks the function unusable there
/// (`exp` blowing up), which also drops `ln` on any x ≤ 0 (it yields NaN/-inf).
const USABLE_CAP: f64 = 1e10;
/// Snap a coefficient to a round value when within this — "2.001x²" → "2x²".
const SNAP: f64 = 0.02;

/// A compact closed-form approximation with its honest error over the domain.
pub struct ClosedForm {
    /// KaTeX-ready, e.g. `f(x) \approx 2x^{2} + 1`.
    pub latex: String,
    /// Largest `|approx − curve|` over the domain.
    pub max_error: f64,
    /// Root-mean-square error over the domain.
    pub rms_error: f64,
}

/// One dictionary entry: how to evaluate it and how it reads in LaTeX (the factor
/// that a coefficient multiplies; empty for the constant term).
struct Basis {
    eval: fn(f64) -> f64,
    latex: &'static str,
}

fn one(_: f64) -> f64 {
    1.0
}
fn linear(x: f64) -> f64 {
    x
}
fn square(x: f64) -> f64 {
    x * x
}
fn cube(x: f64) -> f64 {
    x * x * x
}

const DICTIONARY: &[Basis] = &[
    Basis {
        eval: one,
        latex: "",
    },
    Basis {
        eval: linear,
        latex: "x",
    },
    Basis {
        eval: square,
        latex: "x^{2}",
    },
    Basis {
        eval: cube,
        latex: "x^{3}",
    },
    Basis {
        eval: f64::sin,
        latex: "\\sin x",
    },
    Basis {
        eval: f64::cos,
        latex: "\\cos x",
    },
    Basis {
        eval: f64::exp,
        latex: "e^{x}",
    },
    Basis {
        eval: f64::ln,
        latex: "\\ln x",
    },
];

/// Try to describe the curve as a compact closed form. Returns `Some` only when a
/// sparse basis fit is accurate enough to be trustworthy.
///
/// # Example
/// ```
/// use curve_engine::{approximate, Curve, Knot};
/// // Knots on y = x² with matching slopes make the Hermite spline exactly x².
/// let spline = Curve::new(vec![
///     Knot::with_tangent(0.0, 0.0, 0.0),
///     Knot::with_tangent(1.0, 1.0, 2.0),
///     Knot::with_tangent(2.0, 4.0, 4.0),
/// ])
/// .unwrap()
/// .fit();
/// assert!(approximate::closed_form(&spline).unwrap().latex.contains("x^{2}"));
/// ```
pub fn closed_form(spline: &Spline) -> Option<ClosedForm> {
    let (fit_x, fit_y) = sample(spline, FIT_SAMPLES);
    let (err_x, err_y) = sample(spline, ERROR_SAMPLES);
    let usable = usable_basis(&fit_x);
    let tolerance = REL_TOLERANCE * span(&err_y) + ABS_TOLERANCE;

    // Prefer the fewest terms: search size 1 upward, and take the first size that
    // yields a qualifying fit (lowest RMS among ties).
    for size in 1..=MAX_TERMS.min(usable.len()) {
        let best = combinations(&usable, size)
            .into_iter()
            .filter_map(|subset| fit_subset(&subset, &fit_x, &fit_y, &err_x, &err_y))
            .filter(|form| form.max_error <= tolerance)
            .min_by(|a, b| a.rms_error.total_cmp(&b.rms_error));
        if best.is_some() {
            return best;
        }
    }
    None
}

/// Fit `subset` of the dictionary to the samples, prettify the coefficients, and
/// measure error on the denser grid. `None` if the solve fails or every term
/// rounds away.
fn fit_subset(
    subset: &[usize],
    fit_x: &[f64],
    fit_y: &[f64],
    err_x: &[f64],
    err_y: &[f64],
) -> Option<ClosedForm> {
    let design = DMatrix::from_fn(fit_x.len(), subset.len(), |i, j| {
        (DICTIONARY[subset[j]].eval)(fit_x[i])
    });
    let target = DVector::from_column_slice(fit_y);
    let solved = design.svd(true, true).solve(&target, 1e-12).ok()?;
    let coeffs: Vec<f64> = solved.iter().map(|&c| prettify(c)).collect();

    let approx = |x: f64| -> f64 {
        subset
            .iter()
            .zip(&coeffs)
            .map(|(&idx, &c)| c * (DICTIONARY[idx].eval)(x))
            .sum()
    };
    let (max_error, rms_error) = errors(&approx, err_x, err_y)?;

    let expression = join_terms(
        subset
            .iter()
            .zip(&coeffs)
            .map(|(&idx, &c)| (c, DICTIONARY[idx].latex.to_string())),
    );
    if expression == "0" {
        return None; // every term rounded away — not a real form
    }
    Some(ClosedForm {
        latex: format!("f(x) \\approx {expression}"),
        max_error,
        rms_error,
    })
}

/// Sample the spline at `n` points, split into parallel x and y vectors.
fn sample(spline: &Spline, n: usize) -> (Vec<f64>, Vec<f64>) {
    spline.polyline(n).into_iter().unzip()
}

/// Dictionary indices whose values are finite and bounded over the domain, so a
/// domain-restricted function (`ln` on x ≤ 0, `exp` overflowing) is skipped.
fn usable_basis(fit_x: &[f64]) -> Vec<usize> {
    (0..DICTIONARY.len())
        .filter(|&j| {
            fit_x.iter().all(|&x| {
                let v = (DICTIONARY[j].eval)(x);
                v.is_finite() && v.abs() <= USABLE_CAP
            })
        })
        .collect()
}

/// Max and RMS error of `approx` against the samples; `None` on any non-finite.
fn errors(approx: &impl Fn(f64) -> f64, xs: &[f64], ys: &[f64]) -> Option<(f64, f64)> {
    let mut max_error = 0.0_f64;
    let mut sum_sq = 0.0_f64;
    for (&x, &y) in xs.iter().zip(ys) {
        let value = approx(x);
        if !value.is_finite() {
            return None;
        }
        let diff = (value - y).abs();
        max_error = max_error.max(diff);
        sum_sq += diff * diff;
    }
    Some((max_error, (sum_sq / xs.len() as f64).sqrt()))
}

/// The extent of the y values, floored at 0 (a flat curve has span 0).
fn span(ys: &[f64]) -> f64 {
    let lo = ys.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = ys.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    (hi - lo).max(0.0)
}

/// Snap a coefficient toward a round value (integer, then half) for a clean form;
/// error is remeasured after snapping, so this never hides a poor fit.
fn prettify(c: f64) -> f64 {
    let integer = c.round();
    if (c - integer).abs() < SNAP {
        return integer;
    }
    let half = (c * 2.0).round() / 2.0;
    if (c - half).abs() < SNAP {
        return half;
    }
    (c * 1000.0).round() / 1000.0
}

/// Every `size`-element combination of `items`, in a deterministic order.
fn combinations(items: &[usize], size: usize) -> Vec<Vec<usize>> {
    let n = items.len();
    let mut out = Vec::new();
    if size == 0 || size > n {
        return out;
    }
    let mut c: Vec<usize> = (0..size).collect();
    loop {
        out.push(c.iter().map(|&i| items[i]).collect());
        let mut i = size;
        loop {
            if i == 0 {
                return out;
            }
            i -= 1;
            if c[i] < n - size + i {
                break;
            }
        }
        c[i] += 1;
        for j in i + 1..size {
            c[j] = c[j - 1] + 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Curve, Knot};

    fn fit(knots: Vec<Knot>) -> Spline {
        Curve::new(knots).unwrap().fit()
    }

    /// Knots on a polynomial with matching slopes make the Hermite spline that
    /// exact polynomial, so the approximator should recover it with ~0 error.
    fn parabola() -> Spline {
        fit(vec![
            Knot::with_tangent(0.0, 0.0, 0.0),
            Knot::with_tangent(1.0, 1.0, 2.0),
            Knot::with_tangent(2.0, 4.0, 4.0),
        ])
    }

    #[test]
    fn recovers_a_line() {
        let form = closed_form(&fit(vec![Knot::new(0.0, 1.0), Knot::new(2.0, 5.0)])).unwrap();
        assert!(form.latex.contains("2x"), "{}", form.latex);
        assert!(form.latex.contains('1'), "{}", form.latex); // y = 2x + 1
        assert!(form.max_error < 1e-6, "max error {}", form.max_error);
    }

    #[test]
    fn recovers_a_parabola_as_x_squared() {
        let form = closed_form(&parabola()).unwrap();
        assert!(form.latex.contains("x^{2}"), "{}", form.latex);
        assert!(form.max_error < 1e-6, "max error {}", form.max_error);
    }

    #[test]
    fn prefers_the_fewest_terms() {
        // The line is exactly one term (2x); it must not be dressed up with extras.
        let form = closed_form(&fit(vec![Knot::new(0.0, 0.0), Knot::new(2.0, 4.0)])).unwrap();
        assert_eq!(form.latex, "f(x) \\approx 2x");
    }

    #[test]
    fn a_flat_curve_is_a_constant() {
        let form = closed_form(&fit(vec![Knot::new(0.0, 3.0), Knot::new(2.0, 3.0)])).unwrap();
        assert_eq!(form.latex, "f(x) \\approx 3");
        assert!(form.max_error < 1e-6);
    }

    #[test]
    fn stays_silent_for_a_squiggle() {
        // A high-frequency zigzag is no simple function; offer nothing rather than
        // a false promise.
        let form = closed_form(&fit(vec![
            Knot::new(0.0, 0.0),
            Knot::new(1.0, 2.0),
            Knot::new(2.0, -2.0),
            Knot::new(3.0, 2.0),
            Knot::new(4.0, -2.0),
            Knot::new(5.0, 2.0),
            Knot::new(6.0, 0.0),
        ]));
        assert!(form.is_none());
    }

    #[test]
    fn skips_log_over_negative_x_without_panicking() {
        // Domain spans negatives, so ln is unusable — must not NaN or panic.
        let form = closed_form(&parabola_over_negatives());
        // A parabola about the origin is still x²-ish; the point is no crash.
        let _ = form;
    }

    fn parabola_over_negatives() -> Spline {
        fit(vec![
            Knot::with_tangent(-1.0, 1.0, -2.0),
            Knot::with_tangent(0.0, 0.0, 0.0),
            Knot::with_tangent(1.0, 1.0, 2.0),
        ])
    }

    #[test]
    fn is_deterministic() {
        assert_eq!(
            closed_form(&parabola()).unwrap().latex,
            closed_form(&parabola()).unwrap().latex
        );
    }

    #[test]
    fn combinations_are_complete_and_ordered() {
        assert_eq!(
            combinations(&[0, 1, 2], 2),
            vec![vec![0, 1], vec![0, 2], vec![1, 2]]
        );
        assert_eq!(combinations(&[5, 6], 1), vec![vec![5], vec![6]]);
        assert!(combinations(&[0], 2).is_empty());
    }
}
