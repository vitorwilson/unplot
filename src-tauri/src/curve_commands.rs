use crate::dto::{pairs, render, to_knots, FittedCurve, KnotDto};
use curve_engine::Curve;

/// Resample a raw drawn stroke, fit the shape-preserving spline in the core, and
/// return it for rendering. Errors (as a message) when the stroke is not a valid
/// function — e.g. fewer than two distinct points.
#[tauri::command]
pub(crate) fn fit_curve(samples: Vec<[f64; 2]>, tolerance: f64) -> Result<FittedCurve, String> {
    let knots = curve_engine::resample(&pairs(&samples), tolerance);
    let curve = Curve::new(knots).map_err(|error| error.to_string())?;
    Ok(render(&curve))
}

/// Resume drawing: resample `samples` and append them to the curve described by
/// `existing` knots, joining C¹ (the lift-and-resume gesture). Errors if the new
/// stroke does not continue strictly to the right of the existing curve.
#[tauri::command]
pub(crate) fn extend_curve(
    existing: Vec<KnotDto>,
    samples: Vec<[f64; 2]>,
    tolerance: f64,
) -> Result<FittedCurve, String> {
    let base = Curve::new(to_knots(&existing)).map_err(|error| error.to_string())?;
    let new_knots = curve_engine::resample(&pairs(&samples), tolerance);
    let curve = base.extend(new_knots).map_err(|error| error.to_string())?;
    Ok(render(&curve))
}

/// Re-fit an edited set of knots (dragged points or tangent handles) without
/// resampling — the editing workhorse. Errors if the edit is not a valid
/// function (e.g. a knot dragged past a neighbor's x).
#[tauri::command]
pub(crate) fn refit_curve(knots: Vec<KnotDto>) -> Result<FittedCurve, String> {
    let curve = Curve::new(to_knots(&knots)).map_err(|error| error.to_string())?;
    Ok(render(&curve))
}

#[cfg(test)]
mod tests {
    use super::{extend_curve, fit_curve, refit_curve};
    use crate::dto::dto;

    #[test]
    fn fits_a_drawn_line() {
        let fitted = fit_curve(vec![[0.0, 0.0], [0.5, 1.0], [1.0, 2.0]], 0.05).unwrap();
        assert!(fitted.polyline.len() >= 2);
        assert!((fitted.polyline[0][0] - 0.0).abs() < 1e-9);
        assert!((fitted.polyline.last().unwrap()[0] - 1.0).abs() < 1e-9);
    }

    #[test]
    fn rejects_a_degenerate_stroke() {
        assert!(fit_curve(vec![[0.0, 0.0]], 0.05).is_err());
    }

    #[test]
    fn extends_a_curve_to_the_right() {
        let base = fit_curve(vec![[0.0, 0.0], [1.0, 1.0]], 0.05).unwrap();
        let extended = extend_curve(base.knots, vec![[2.0, 0.0], [3.0, 1.0]], 0.05).unwrap();
        // The combined curve now spans [0, 3].
        assert!((extended.polyline[0][0] - 0.0).abs() < 1e-9);
        assert!((extended.polyline.last().unwrap()[0] - 3.0).abs() < 1e-9);
    }

    #[test]
    fn rejects_a_resume_that_goes_backward() {
        let base = fit_curve(vec![[0.0, 0.0], [2.0, 1.0]], 0.05).unwrap();
        assert!(extend_curve(base.knots, vec![[1.0, 0.0], [1.5, 0.0]], 0.05).is_err());
    }

    #[test]
    fn refit_honors_a_user_tangent() {
        // A flat tangent at both ends over [0, 1] is the smoothstep 3t²−2t³,
        // which passes through 0.5 at the midpoint.
        let knots = vec![dto(0.0, 0.0, Some(0.0)), dto(1.0, 1.0, Some(0.0))];
        let fitted = refit_curve(knots).unwrap();
        let mid = fitted.polyline[fitted.polyline.len() / 2];
        assert!((mid[1] - 0.5).abs() < 0.05, "midpoint y was {}", mid[1]);
    }

    #[test]
    fn refit_reports_the_slope_at_each_knot() {
        // A straight run of knots -> every effective slope equals the gradient.
        let knots = vec![
            dto(0.0, 0.0, None),
            dto(1.0, 2.0, None),
            dto(2.0, 4.0, None),
        ];
        let fitted = refit_curve(knots).unwrap();
        for knot in fitted.knots {
            assert!((knot.slope - 2.0).abs() < 1e-9, "slope was {}", knot.slope);
        }
    }

    #[test]
    fn refit_rejects_a_knot_dragged_past_its_neighbor() {
        let knots = vec![
            dto(0.0, 0.0, None),
            dto(2.0, 1.0, None),
            dto(1.0, 1.0, None),
        ];
        assert!(refit_curve(knots).is_err());
    }
}
