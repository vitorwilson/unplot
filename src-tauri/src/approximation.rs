use crate::dto::{Approximation, POLYLINE_POINTS};

/// The closed-form "prettier function" for a spline (Phase 7), with error as a
/// fraction of the y-range, or `None` when no trustworthy form exists. Used for a
/// derivative/integral, whose spline is itself the exact (piecewise-polynomial)
/// result — so sampling it, rather than any knots, is the right target.
pub(crate) fn approximate_curve(spline: &curve_engine::Spline) -> Option<Approximation> {
    normalize(
        curve_engine::approximate::closed_form(spline),
        y_span(spline),
    )
}

/// Express a closed form's error as a fraction of the curve's y-range, so the UI
/// can show "max 0.4%". A near-flat curve floors the range to avoid dividing by
/// zero.
pub(crate) fn normalize(
    form: Option<curve_engine::approximate::ClosedForm>,
    range: f64,
) -> Option<Approximation> {
    form.map(|form| {
        let range = range.max(1e-6);
        Approximation {
            latex: form.latex,
            max_error: form.max_error / range,
            rms_error: form.rms_error / range,
        }
    })
}

/// The extent of the spline's y values, used to express fit error as a fraction.
pub(crate) fn y_span(spline: &curve_engine::Spline) -> f64 {
    let ys = spline.polyline(POLYLINE_POINTS);
    let lo = ys.iter().map(|&(_, y)| y).fold(f64::INFINITY, f64::min);
    let hi = ys.iter().map(|&(_, y)| y).fold(f64::NEG_INFINITY, f64::max);
    (hi - lo).max(0.0)
}
