//! Strategy 2: a free-frequency sinusoid `A·sin(ωx) + B·cos(ωx) + C`. ω is found
//! by sweeping a periodogram over the fit points then refining locally, so a
//! drawn wave of arbitrary frequency is caught where the fixed frequency-1 trig
//! basis would miss it.

use super::{
    candidate_of, prettify, solve, Candidate, ABS_TOLERANCE, OMEGA_CAP, REFINE_STEPS,
    SAMPLES_PER_PERIOD, SWEEP_STEPS,
};
use crate::symbolic::{Expr, Term};
use std::f64::consts::PI;

/// The best single sinusoid `A·sin(ωx) + B·cos(ωx) + C` for a curve whose shape
/// is a wave of arbitrary frequency, or `None`. ω is found by sweeping (a
/// periodogram) then refining and snapping toward a round value.
pub(super) fn sinusoid_candidate(
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
    let wave = Expr::Sum(vec![
        Term::Power {
            coeff: coeffs[0],
            power: 0,
        },
        Term::Sin {
            coeff: coeffs[1],
            omega,
        },
        Term::Cos {
            coeff: coeffs[2],
            omega,
        },
    ]);
    candidate_of(wave, err_x, err_y, tolerance)
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

#[cfg(test)]
mod tests {
    use super::super::closed_form;
    use crate::{Curve, Knot, Spline};
    use std::f64::consts::PI;

    /// A spline that closely follows `f` by pinning each knot's value and slope.
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
}
