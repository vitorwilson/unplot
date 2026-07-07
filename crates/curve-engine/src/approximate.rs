//! The "prettier function" (Phase 7): when the drawn curve is *basically* a
//! simple function, offer a compact closed form beside the exact piecewise
//! output — never instead of it. Two strategies compete, both error-gated:
//!
//! 1. a sparse least-squares fit against a fixed basis dictionary (`{1, x, x²,
//!    x³, sin x, cos x, eˣ, ln x}`) — nails "basically x²" or "basically eˣ";
//! 2. a free-frequency sinusoid `A·sin(ωx) + B·cos(ωx) + C`, found by sweeping ω
//!    (a periodogram) — catches a drawn wave of any frequency, which the fixed
//!    frequency-1 trig basis misses.
//!
//! The simplest trustworthy form wins; if neither clears the error gate the
//! result is `None` and the caller keeps the exact output. Reports honest max/RMS
//! error. FOSS-only, pure-Rust (nalgebra); no CAS. Deterministic.

use crate::coeffs::{fmt_num, join_terms, DISPLAY_EPS};
use crate::knot::Knot;
use crate::spline::Spline;
use nalgebra::{DMatrix, DVector};
use std::f64::consts::PI;

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
/// Snap a coefficient (or frequency) to a round value when within this.
const SNAP: f64 = 0.02;
/// Coarse frequencies tried in the sinusoid sweep, then refined locally.
const SWEEP_STEPS: usize = 256;
const REFINE_STEPS: usize = 40;
/// Never sweep past this angular frequency, however dense the sampling.
const OMEGA_CAP: f64 = 24.0;
/// Require at least this many samples per period, so a claimed frequency is
/// actually resolved by the sampling rather than aliased.
const SAMPLES_PER_PERIOD: f64 = 6.0;

/// A compact closed-form approximation with its honest error over the domain.
pub struct ClosedForm {
    /// KaTeX-ready, e.g. `f(x) \approx 2x^{2} + 1`.
    pub latex: String,
    /// Largest `|approx − curve|` over the domain.
    pub max_error: f64,
    /// Root-mean-square error over the domain.
    pub rms_error: f64,
}

/// A fitted candidate before it competes with the others: its rendered form, its
/// error, and how many terms it reads as (fewer is prettier).
struct Candidate {
    latex: String,
    max_error: f64,
    rms_error: f64,
    terms: usize,
}

/// Try to describe the curve as a compact closed form. Returns `Some` only when a
/// strategy fits accurately enough to be trustworthy; the simplest such form
/// (fewest terms, then lowest RMS) is returned.
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
    fit_closed_form(&fit_x, &fit_y, &err_x, &err_y, spline.domain())
}

/// Recover a closed form from the user's exact knots (typed points or dragged
/// handles) rather than the smoothed spline. This is the right target for the
/// *drawn* curve: the shape-preserving PCHIP fit zeroes the slope at a sparse
/// wave's peaks and runs nearly straight to the next knot, so fitting the spline
/// would miss a cosine the points plainly trace (commit 25b7ae3 follow-up). The
/// knots are the ground truth, so fit and error are both measured at them.
///
/// # Example
/// ```
/// use curve_engine::{approximate, Knot};
/// use std::f64::consts::PI;
/// // Five exact points over one period of cos(x) — a natural thing to type.
/// let knots: Vec<Knot> = [0.0, PI / 2.0, PI, 3.0 * PI / 2.0, 2.0 * PI]
///     .iter()
///     .map(|&x| Knot::new(x, x.cos()))
///     .collect();
/// assert!(approximate::closed_form_of_knots(&knots).unwrap().latex.contains("\\cos"));
/// ```
pub fn closed_form_of_knots(knots: &[Knot]) -> Option<ClosedForm> {
    let xs: Vec<f64> = knots.iter().map(|k| k.x).collect();
    let ys: Vec<f64> = knots.iter().map(|k| k.y).collect();
    let domain = (xs[0], xs[xs.len() - 1]);
    fit_closed_form(&xs, &ys, &xs, &ys, domain)
}

/// The best trustworthy closed form for samples fitted over `domain`: the two
/// strategies compete and the simplest that clears the error gate (fewest terms,
/// then lowest RMS) wins, or `None`.
fn fit_closed_form(
    fit_x: &[f64],
    fit_y: &[f64],
    err_x: &[f64],
    err_y: &[f64],
    domain: (f64, f64),
) -> Option<ClosedForm> {
    let tolerance = REL_TOLERANCE * span(err_y) + ABS_TOLERANCE;
    [
        basis_candidate(fit_x, fit_y, err_x, err_y, tolerance),
        sinusoid_candidate(domain, fit_x, fit_y, err_x, err_y, tolerance),
    ]
    .into_iter()
    .flatten()
    .min_by(|a, b| {
        a.terms
            .cmp(&b.terms)
            .then(a.rms_error.total_cmp(&b.rms_error))
    })
    .map(|best| ClosedForm {
        latex: best.latex,
        max_error: best.max_error,
        rms_error: best.rms_error,
    })
}

// --- Strategy 1: sparse fixed-basis least squares -------------------------------

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

/// The fewest-term basis fit whose error clears the gate, or `None`.
fn basis_candidate(
    fit_x: &[f64],
    fit_y: &[f64],
    err_x: &[f64],
    err_y: &[f64],
    tolerance: f64,
) -> Option<Candidate> {
    let usable = usable_basis(fit_x);
    // A clean fit is only *evidence* of a shape when it has more points than free
    // terms: an n-point set passes exactly through any n-term basis, so allowing
    // size == n would "recognize" noise and, on ties, pick an ugly exact fit (a
    // 2-point line read as `1 + 0.5x³`). Require at least one residual degree of
    // freedom — which only bites the knots path, as the spline path samples 120.
    let max_terms = MAX_TERMS
        .min(usable.len())
        .min(fit_x.len().saturating_sub(1));
    for size in 1..=max_terms {
        let best = combinations(&usable, size)
            .into_iter()
            .filter_map(|subset| fit_basis_subset(&subset, fit_x, fit_y, err_x, err_y))
            .filter(|form| form.max_error <= tolerance)
            .min_by(|a, b| a.rms_error.total_cmp(&b.rms_error));
        if best.is_some() {
            return best; // fewer terms wins, so the first non-empty size is best
        }
    }
    None
}

/// Fit `subset` of the dictionary to the samples, prettify the coefficients, and
/// measure error on the denser grid. `None` if the solve fails or every term
/// rounds away.
fn fit_basis_subset(
    subset: &[usize],
    fit_x: &[f64],
    fit_y: &[f64],
    err_x: &[f64],
    err_y: &[f64],
) -> Option<Candidate> {
    let coeffs = solve(fit_x, fit_y, subset.len(), |i, j| {
        (DICTIONARY[subset[j]].eval)(fit_x[i])
    })?;
    let approx = |x: f64| -> f64 {
        subset
            .iter()
            .zip(&coeffs)
            .map(|(&idx, &c)| c * (DICTIONARY[idx].eval)(x))
            .sum()
    };
    let (max_error, rms_error) = errors(&approx, err_x, err_y)?;
    let pairs: Vec<(f64, String)> = subset
        .iter()
        .zip(&coeffs)
        .map(|(&idx, &c)| (c, DICTIONARY[idx].latex.to_string()))
        .collect();
    candidate(&pairs, max_error, rms_error)
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

// --- Strategy 2: free-frequency sinusoid ---------------------------------------

/// The best single sinusoid `A·sin(ωx) + B·cos(ωx) + C` for a curve whose shape
/// is a wave of arbitrary frequency, or `None`. ω is found by sweeping (a
/// periodogram) then refining and snapping toward a round value.
fn sinusoid_candidate(
    domain: (f64, f64),
    fit_x: &[f64],
    fit_y: &[f64],
    err_x: &[f64],
    err_y: &[f64],
    tolerance: f64,
) -> Option<Candidate> {
    let (a, b) = domain;
    let length = b - a;
    if length <= ABS_TOLERANCE {
        return None;
    }
    // From "half a wave spans the domain" up to what the sampling can resolve —
    // capped by the actual point count so a claimed frequency is not aliased.
    let omega_min = PI / length;
    let omega_max = (PI * fit_x.len() as f64 / (SAMPLES_PER_PERIOD * length)).min(OMEGA_CAP);
    if omega_max <= omega_min {
        return None;
    }

    let coarse = best_frequency(fit_x, fit_y, omega_min, omega_max, SWEEP_STEPS)?;
    let step = (omega_max - omega_min) / SWEEP_STEPS as f64;
    let refined = best_frequency(
        fit_x,
        fit_y,
        (coarse - step).max(omega_min),
        (coarse + step).min(omega_max),
        REFINE_STEPS,
    )?;
    let omega = prettify(refined);

    let coeffs: Vec<f64> = solve_sinusoid(fit_x, fit_y, omega)?
        .iter()
        .map(|&c| prettify(c))
        .collect();
    let approx = |x: f64| coeffs[0] + coeffs[1] * (omega * x).sin() + coeffs[2] * (omega * x).cos();
    let (max_error, rms_error) = errors(&approx, err_x, err_y)?;
    if max_error > tolerance {
        return None;
    }
    let pairs = vec![
        (coeffs[0], String::new()),
        (coeffs[1], format!("\\sin({})", angle(omega))),
        (coeffs[2], format!("\\cos({})", angle(omega))),
    ];
    candidate(&pairs, max_error, rms_error)
}

/// The frequency in `[lo, hi]` (over `steps` samples) with the smallest sinusoid
/// residual on the fit points.
fn best_frequency(fit_x: &[f64], fit_y: &[f64], lo: f64, hi: f64, steps: usize) -> Option<f64> {
    let mut best: Option<(f64, f64)> = None; // (residual, omega)
    for i in 0..=steps {
        let omega = lo + (hi - lo) * i as f64 / steps as f64;
        if let Some(residual) = sinusoid_residual(fit_x, fit_y, omega) {
            let improves = match best {
                Some((r, _)) => residual < r,
                None => true,
            };
            if improves {
                best = Some((residual, omega));
            }
        }
    }
    best.map(|(_, omega)| omega)
}

/// Sum of squared residuals of the best `{1, sin(ωx), cos(ωx)}` fit at `omega`.
fn sinusoid_residual(fit_x: &[f64], fit_y: &[f64], omega: f64) -> Option<f64> {
    let coeffs = solve_sinusoid(fit_x, fit_y, omega)?;
    let sse = fit_x
        .iter()
        .zip(fit_y)
        .map(|(&x, &y)| {
            let v = coeffs[0] + coeffs[1] * (omega * x).sin() + coeffs[2] * (omega * x).cos();
            (v - y) * (v - y)
        })
        .sum();
    Some(sse)
}

/// Least-squares `[C, A, B]` for `C + A·sin(ωx) + B·cos(ωx)`.
fn solve_sinusoid(fit_x: &[f64], fit_y: &[f64], omega: f64) -> Option<Vec<f64>> {
    solve(fit_x, fit_y, 3, |i, j| match j {
        0 => 1.0,
        1 => (omega * fit_x[i]).sin(),
        _ => (omega * fit_x[i]).cos(),
    })
}

/// The `ωx` argument of a trig term: `x` for ω = 1, else `2x`, `0.5x`, …
fn angle(omega: f64) -> String {
    if (omega - 1.0).abs() < DISPLAY_EPS {
        "x".to_string()
    } else {
        format!("{}x", fmt_num(omega))
    }
}

// --- Shared helpers ------------------------------------------------------------

/// Solve the least-squares system whose design matrix is `columns` wide with
/// entries `entry(row, col)`, against `fit_y`. `None` if the SVD solve fails.
fn solve(
    fit_x: &[f64],
    fit_y: &[f64],
    columns: usize,
    entry: impl Fn(usize, usize) -> f64,
) -> Option<Vec<f64>> {
    let design = DMatrix::from_fn(fit_x.len(), columns, entry);
    let target = DVector::from_column_slice(fit_y);
    let solved = design.svd(true, true).solve(&target, 1e-12).ok()?;
    Some(solved.iter().copied().collect())
}

/// Build a candidate from `(coefficient, factor)` term pairs — its rendered LaTeX
/// and how many terms survive. `None` when every term rounds away (`"0"`).
fn candidate(pairs: &[(f64, String)], max_error: f64, rms_error: f64) -> Option<Candidate> {
    let terms = pairs.iter().filter(|(c, _)| c.abs() >= DISPLAY_EPS).count();
    let expression = join_terms(pairs.iter().cloned());
    if expression == "0" {
        return None;
    }
    Some(Candidate {
        latex: format!("f(x) \\approx {expression}"),
        max_error,
        rms_error,
        terms,
    })
}

/// Sample the spline at `n` points, split into parallel x and y vectors.
fn sample(spline: &Spline, n: usize) -> (Vec<f64>, Vec<f64>) {
    spline.polyline(n).into_iter().unzip()
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

/// Snap a value toward a round one (integer, then half) for a clean form; error
/// is remeasured after snapping, so this never hides a poor fit.
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

    /// A spline that closely follows `f` by pinning each knot's value and slope,
    /// so tests can feed the approximator a known function.
    fn spline_of(
        f: impl Fn(f64) -> f64,
        df: impl Fn(f64) -> f64,
        a: f64,
        b: f64,
        n: usize,
    ) -> Spline {
        let knots = (0..n)
            .map(|i| {
                let x = a + (b - a) * i as f64 / (n - 1) as f64;
                Knot::with_tangent(x, f(x), df(x))
            })
            .collect();
        Curve::new(knots).unwrap().fit()
    }

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
    fn recovers_a_higher_frequency_sine() {
        // sin(2x) over one period [0, π]: the fixed frequency-1 basis can't express
        // it, but the sinusoid sweep finds ω = 2.
        let spline = spline_of(|x| (2.0 * x).sin(), |x| 2.0 * (2.0 * x).cos(), 0.0, PI, 13);
        let form = closed_form(&spline).unwrap();
        assert!(form.latex.contains("\\sin(2x)"), "{}", form.latex);
        assert!(form.max_error < 0.03, "max error {}", form.max_error);
    }

    #[test]
    fn recovers_a_shifted_cosine_wave() {
        // 3·cos(1.5x): a wave the fixed basis misses on both amplitude and freq.
        let spline = spline_of(
            |x| 3.0 * (1.5 * x).cos(),
            |x| -4.5 * (1.5 * x).sin(),
            0.0,
            4.0,
            15,
        );
        let form = closed_form(&spline).unwrap();
        assert!(form.latex.contains("\\cos(1.5x)"), "{}", form.latex);
        assert!(form.latex.contains('3'), "{}", form.latex);
    }

    /// Points a user would *type* — coordinates only, no tangents — for the given
    /// function over `[a, b]`. The PCHIP fit these produce is a poor wave (peaks
    /// flattened), which is exactly why the knots path fits the points directly.
    fn typed(f: impl Fn(f64) -> f64, a: f64, b: f64, n: usize) -> Vec<Knot> {
        (0..n)
            .map(|i| {
                let x = a + (b - a) * i as f64 / (n - 1) as f64;
                Knot::new(x, f(x))
            })
            .collect()
    }

    #[test]
    fn recovers_a_cosine_from_five_typed_points() {
        // The reported bug: five exact points over one period of cos(x). Fitting the
        // smoothed spline misses it (peaks flattened ~21%); fitting the knots nails
        // it, because frequency-1 cos is one basis term.
        let form = closed_form_of_knots(&typed(f64::cos, 0.0, 2.0 * PI, 5)).unwrap();
        assert!(form.latex.contains("\\cos"), "{}", form.latex);
        assert!(form.max_error < 1e-6, "max error {}", form.max_error);
    }

    #[test]
    fn recovers_a_scaled_shifted_cosine_from_typed_points() {
        let form = closed_form_of_knots(&typed(|x| 2.0 * x.cos() + 1.0, 0.0, 2.0 * PI, 9)).unwrap();
        assert!(form.latex.contains("2\\cos"), "{}", form.latex);
        assert!(form.latex.contains('1'), "{}", form.latex); // the +1 offset
    }

    #[test]
    fn knots_path_stays_silent_for_random_points() {
        // Three arbitrary points fit a 3-term basis exactly, but that is a tautology,
        // not a discovery — the residual-degree-of-freedom guard must reject it.
        let form = closed_form_of_knots(&[
            Knot::new(0.0, 0.3),
            Knot::new(1.0, 2.7),
            Knot::new(2.0, -1.1),
        ]);
        assert!(form.is_none(), "{:?}", form.map(|f| f.latex));
    }

    #[test]
    fn knots_path_stays_silent_for_a_squiggle() {
        let form = closed_form_of_knots(&typed(
            |x| 2.0 * (3.0 * x).sin() + (7.0 * x).cos(),
            0.0,
            2.0,
            7,
        ));
        assert!(form.is_none(), "{:?}", form.map(|f| f.latex));
    }

    #[test]
    fn stays_silent_for_a_squiggle() {
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
        let _ = closed_form(&fit(vec![
            Knot::with_tangent(-1.0, 1.0, -2.0),
            Knot::with_tangent(0.0, 0.0, 0.0),
            Knot::with_tangent(1.0, 1.0, 2.0),
        ]));
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
