use crate::approximation::approximate_curve;
use crate::dto::{to_knots, CalcCurve, CalcOp, KnotDto, POLYLINE_POINTS};
use curve_engine::symbolic::Expr;
use curve_engine::Curve;

/// Fit `knots`, apply each calculus `op` left to right, and return the resulting
/// curve for display. When the drawn curve is recognized as a clean function, the
/// calculus is done exactly on that closed form (d/dx of x³ is 3x², not the lumpy
/// derivative of the smoothed spline); otherwise it falls back to the numeric
/// per-segment result. An empty `ops` returns the drawn curve unchanged.
#[tauri::command]
pub(crate) fn apply_calculus(knots: Vec<KnotDto>, ops: Vec<CalcOp>) -> Result<CalcCurve, String> {
    let curve = Curve::new(to_knots(&knots)).map_err(|error| error.to_string())?;
    let spline = curve.fit();
    Ok(symbolic_calculus(&curve, &spline, &ops).unwrap_or_else(|| numeric_calculus(&spline, &ops)))
}

/// Calculus on the recognized closed form of the drawn curve, or `None` to fall
/// back to numeric — when nothing is recognized, an op has no closed form here (a
/// rational integral), or the result is non-finite over the domain.
fn symbolic_calculus(
    curve: &Curve,
    spline: &curve_engine::Spline,
    ops: &[CalcOp],
) -> Option<CalcCurve> {
    let (start, end) = spline.domain();
    let mut expr = curve_engine::approximate::closed_form_of_knots(curve.knots())?.expr;
    for op in ops {
        expr = match op {
            CalcOp::Differentiate => expr.differentiate(),
            CalcOp::Integrate => expr.integrate(start)?,
        };
    }
    let polyline = symbolic_polyline(&expr, start, end)?;
    let form = expr.to_latex();
    Some(CalcCurve {
        polyline,
        summary: form.clone(),
        latex: format!("f(x) = {form}"),
        desmos: form,
        wolfram: expr.to_wolfram(),
        approximation: None, // the result is exact — no separate "prettier" form
        exact: true,
    })
}

/// Sample a symbolic result across `[start, end]`; `None` if it is non-finite
/// anywhere (a pole or a log's edge), so the caller falls back to numeric.
fn symbolic_polyline(expr: &Expr, start: f64, end: f64) -> Option<Vec<[f64; 2]>> {
    (0..POLYLINE_POINTS)
        .map(|i| {
            let x = start + (end - start) * i as f64 / (POLYLINE_POINTS - 1) as f64;
            let y = expr.eval(x);
            y.is_finite().then_some([x, y])
        })
        .collect()
}

/// The numeric per-segment calculus: exact on the fitted spline, and the only
/// path for a curve no closed form describes. Its derivative has corners at the
/// knots and its integral is smooth, which the UI notes (`exact: false`).
fn numeric_calculus(spline: &curve_engine::Spline, ops: &[CalcOp]) -> CalcCurve {
    let mut result = spline.clone();
    for op in ops {
        result = match op {
            CalcOp::Differentiate => curve_engine::calculus::differentiate(&result),
            CalcOp::Integrate => curve_engine::calculus::integrate(&result),
        };
    }
    CalcCurve {
        polyline: result
            .polyline(POLYLINE_POINTS)
            .iter()
            .map(|&(x, y)| [x, y])
            .collect(),
        summary: curve_engine::latex::summary(&result),
        latex: curve_engine::latex::piecewise(&result),
        desmos: curve_engine::export::desmos(&result),
        wolfram: curve_engine::export::wolfram(&result),
        approximation: approximate_curve(&result),
        exact: false,
    }
}

#[cfg(test)]
mod tests {
    use super::apply_calculus;
    use crate::dto::{dto, CalcOp, KnotDto};

    /// Typed points of `f` at each integer `x` in `[lo, hi]`.
    fn integer_points(lo: i32, hi: i32, f: impl Fn(f64) -> f64) -> Vec<KnotDto> {
        (lo..=hi)
            .map(|x| dto(x as f64, f(x as f64), None))
            .collect()
    }

    #[test]
    fn apply_calculus_differentiates_a_line_to_a_constant() {
        // f(x) = 2x is recognized, so its derivative is the exact constant 2 — a
        // clean symbolic result, not the numeric piecewise one.
        let result = apply_calculus(
            vec![dto(0.0, 0.0, None), dto(2.0, 4.0, None)],
            vec![CalcOp::Differentiate],
        )
        .unwrap();
        assert!(result.polyline.iter().all(|&[_, y]| (y - 2.0).abs() < 1e-9));
        assert!(
            result.exact,
            "a recognized curve differentiates symbolically"
        );
        assert_eq!(result.latex, "f(x) = 2", "{}", result.latex);
        assert!(result.approximation.is_none(), "exact — no approximation");
    }

    #[test]
    fn apply_calculus_differentiates_a_typed_cubic_to_a_clean_parabola() {
        // The reported bug: d/dx of typed x³ points was a lumpy numeric derivative.
        // Recognized as x³, it differentiates to exactly 3x² (a smooth parabola).
        let result = apply_calculus(
            integer_points(-3, 3, |x| x.powi(3)),
            vec![CalcOp::Differentiate],
        )
        .unwrap();
        assert!(result.exact);
        assert_eq!(result.latex, "f(x) = 3x^{2}", "{}", result.latex);
    }

    #[test]
    fn apply_calculus_integrates_a_typed_cubic_without_a_bogus_pretty_form() {
        // The old numeric path offered a wrong closed form (`… + cos x`) for ∫x³,
        // whose true value x⁴/4 is outside the basis. Symbolically it is exact and
        // carries no misleading approximation.
        let result = apply_calculus(
            integer_points(-3, 3, |x| x.powi(3)),
            vec![CalcOp::Integrate],
        )
        .unwrap();
        assert!(result.exact);
        assert!(result.latex.contains("0.25x^{4}"), "{}", result.latex);
        assert!(result.approximation.is_none(), "no bogus approximation");
    }

    #[test]
    fn apply_calculus_falls_back_to_numeric_for_an_unrecognized_squiggle() {
        // A curve no closed form describes still differentiates numerically
        // (piecewise), flagged not-exact so the UI keeps its "corners" note.
        let knots = vec![
            dto(0.0, 0.0, None),
            dto(1.0, 2.0, None),
            dto(2.0, -2.0, None),
            dto(3.0, 2.0, None),
            dto(4.0, -2.0, None),
            dto(5.0, 2.0, None),
            dto(6.0, 0.0, None),
        ];
        let result = apply_calculus(knots, vec![CalcOp::Differentiate]).unwrap();
        assert!(!result.exact, "an unrecognized curve uses the numeric path");
        assert!(result.latex.contains("\\begin{cases}"));
    }

    #[test]
    fn apply_calculus_chains_integrate_then_differentiate_back_to_the_curve() {
        // FTC end-to-end: d/dx ∫ f = f, so the chained polyline matches the drawn
        // curve's within tolerance.
        let knots = vec![
            dto(0.0, 0.0, None),
            dto(1.0, 2.0, None),
            dto(2.0, -1.0, None),
        ];
        let original = apply_calculus(knots.clone(), vec![]).unwrap();
        let recovered =
            apply_calculus(knots, vec![CalcOp::Integrate, CalcOp::Differentiate]).unwrap();
        for (o, r) in original.polyline.iter().zip(&recovered.polyline) {
            assert!((o[1] - r[1]).abs() < 1e-6, "{} vs {}", o[1], r[1]);
        }
    }
}
