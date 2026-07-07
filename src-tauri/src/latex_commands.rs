use crate::approximation::{normalize, y_span};
use crate::dto::{to_knots, CurveLatex, KnotDto};
use curve_engine::Curve;

#[tauri::command]
pub(crate) fn curve_latex(knots: Vec<KnotDto>) -> Result<CurveLatex, String> {
    let curve = Curve::new(to_knots(&knots)).map_err(|error| error.to_string())?;
    let spline = curve.fit();
    Ok(CurveLatex {
        summary: curve_engine::latex::summary(&spline),
        latex: curve_engine::latex::piecewise(&spline),
        desmos: curve_engine::export::desmos(&spline),
        wolfram: curve_engine::export::wolfram(&spline),
        // Fit the user's exact knots, not the smoothed spline: PCHIP flattens a
        // sparse wave's peaks, so sampling the spline misses a cosine the typed
        // points plainly trace (see approximate::closed_form_of_knots).
        approximation: normalize(
            curve_engine::approximate::closed_form_of_knots(curve.knots()),
            y_span(&spline),
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::curve_latex;
    use crate::dto::dto;

    #[test]
    fn curve_latex_returns_every_copy_format() {
        let result = curve_latex(vec![dto(0.0, 0.0, None), dto(2.0, 4.0, None)]).unwrap();
        assert_eq!(result.summary, "1-segment spline over [0, 2]");
        assert!(result.latex.contains("\\begin{cases}"));
        assert!(result.desmos.contains("\\left\\{"));
        assert!(result.wolfram.contains("Piecewise[{{"));
    }

    #[test]
    fn curve_latex_offers_a_closed_form_for_a_simple_curve() {
        // y = 2x is exactly one basis term, so a trustworthy form is offered.
        let result = curve_latex(vec![dto(0.0, 0.0, None), dto(2.0, 4.0, None)]).unwrap();
        let approx = result
            .approximation
            .expect("a line should get a closed form");
        assert!(approx.latex.contains("2x"), "{}", approx.latex);
        assert!(
            approx.max_error < 0.01,
            "relative error {}",
            approx.max_error
        );
    }

    #[test]
    fn curve_latex_recognizes_typed_cosine_points() {
        // Regression: five exact points over one period of cos(x), typed with no
        // tangents. The smoothed spline flattens the peaks, so the closed form must
        // be fitted from the knots for the app to recognize the wave.
        let quarter = std::f64::consts::PI / 2.0;
        let knots = (0..5)
            .map(|i| {
                let x = i as f64 * quarter;
                dto(x, x.cos(), None)
            })
            .collect();
        let approx = curve_latex(knots)
            .unwrap()
            .approximation
            .expect("typed cosine points should get a closed form");
        assert!(approx.latex.contains("\\cos"), "{}", approx.latex);
    }

    #[test]
    fn curve_latex_recognizes_a_typed_hyperbola() {
        // Phase 7 rational strategy: y = 1/x is a pole-shaped curve no polynomial or
        // wave can express, so the closed form should be the fraction 1/x.
        let knots = (0..8)
            .map(|i| {
                let x = 0.5 + i as f64 * 0.5;
                dto(x, 1.0 / x, None)
            })
            .collect();
        let approx = curve_latex(knots)
            .unwrap()
            .approximation
            .expect("a typed hyperbola should get a closed form");
        assert_eq!(
            approx.latex, "f(x) \\approx \\frac{1}{x}",
            "{}",
            approx.latex
        );
    }
}
