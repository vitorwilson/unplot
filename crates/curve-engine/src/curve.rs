use crate::error::CurveError;
use crate::knot::Knot;
use crate::spline::{fit_spline, Spline};

/// An editable, single-valued curve: knots ordered left to right with strictly
/// increasing x.
///
/// This is the source of truth the UI edits and the fitter, calculus, and
/// serialization all derive from — not a symbolic guess (docs/PLAN.md, Locked
/// decisions). Its invariants (≥2 knots, finite coordinates, strictly increasing
/// x) are enforced at construction, so a `Curve` value is always a valid function.
#[derive(Debug, Clone, PartialEq)]
pub struct Curve {
    knots: Vec<Knot>,
}

impl Curve {
    /// Build a curve from left-to-right knots, enforcing the function invariants.
    ///
    /// # Example
    /// ```
    /// use curve_engine::{Curve, Knot};
    /// let c = Curve::new(vec![Knot::new(0.0, 0.0), Knot::new(1.0, 2.0)]).unwrap();
    /// assert_eq!(c.domain(), (0.0, 1.0));
    /// ```
    pub fn new(knots: Vec<Knot>) -> Result<Self, CurveError> {
        check_count(&knots)?;
        check_finite(&knots)?;
        check_strictly_increasing(&knots)?;
        Ok(Curve { knots })
    }

    /// The closed interval `[x_first, x_last]` the curve is defined on. There is
    /// no extrapolation beyond it (Locked decision: honest domain).
    pub fn domain(&self) -> (f64, f64) {
        // `new` guarantees at least two knots, so these indices are always valid.
        (self.knots[0].x, self.knots[self.knots.len() - 1].x)
    }

    /// The knots, left to right.
    pub fn knots(&self) -> &[Knot] {
        &self.knots
    }

    /// Fit the shape-preserving cubic Hermite spline for this curve. Cheap and
    /// deterministic — call it whenever the knots change.
    ///
    /// # Example
    /// ```
    /// use curve_engine::{Curve, Knot};
    /// let spline = Curve::new(vec![Knot::new(0.0, 0.0), Knot::new(1.0, 1.0)])
    ///     .unwrap()
    ///     .fit();
    /// assert_eq!(spline.domain(), (0.0, 1.0));
    /// ```
    pub fn fit(&self) -> Spline {
        fit_spline(&self.knots)
    }

    /// Resume drawing: append `additional` knots to the right, pinning the join
    /// so the combined curve stays C¹ — the new stroke begins with the slope the
    /// previous stroke ended on (docs/PLAN.md: "Drawing in pieces"). The new
    /// knots must continue strictly to the right, or construction fails.
    ///
    /// # Example
    /// ```
    /// use curve_engine::{Curve, Knot};
    /// let base = Curve::new(vec![Knot::new(0.0, 0.0), Knot::new(1.0, 1.0)]).unwrap();
    /// let joined = base.extend(vec![Knot::new(2.0, 0.0)]).unwrap();
    /// assert_eq!(joined.domain(), (0.0, 2.0));
    /// ```
    pub fn extend(&self, additional: Vec<Knot>) -> Result<Curve, CurveError> {
        let end_slope = self.fit().end_slope();
        let mut knots = self.knots.clone();
        // Pin the current last knot to the slope the stroke ended on, so
        // re-fitting neither kinks the join nor re-shapes the previous stroke.
        if let Some(last) = knots.last_mut() {
            last.tangent = Some(end_slope);
        }
        knots.extend(additional);
        Curve::new(knots)
    }
}

fn check_count(knots: &[Knot]) -> Result<(), CurveError> {
    if knots.len() < 2 {
        return Err(CurveError::TooFewKnots { count: knots.len() });
    }
    Ok(())
}

fn check_finite(knots: &[Knot]) -> Result<(), CurveError> {
    for (index, knot) in knots.iter().enumerate() {
        if !knot.x.is_finite() || !knot.y.is_finite() {
            return Err(CurveError::NonFiniteCoordinate {
                index,
                x: knot.x,
                y: knot.y,
            });
        }
        if let Some(tangent) = knot.tangent {
            if !tangent.is_finite() {
                return Err(CurveError::NonFiniteTangent { index, tangent });
            }
        }
    }
    Ok(())
}

fn check_strictly_increasing(knots: &[Knot]) -> Result<(), CurveError> {
    for (i, pair) in knots.windows(2).enumerate() {
        if pair[1].x <= pair[0].x {
            return Err(CurveError::NonIncreasingX {
                index: i + 1,
                prev_x: pair[0].x,
                x: pair[1].x,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<Knot> {
        vec![
            Knot::new(0.0, 0.0),
            Knot::new(1.0, 2.0),
            Knot::new(3.0, -1.0),
        ]
    }

    #[test]
    fn builds_from_strictly_increasing_knots() {
        let curve = Curve::new(sample()).expect("valid knots");
        assert_eq!(curve.knots().len(), 3);
        assert_eq!(curve.domain(), (0.0, 3.0));
    }

    #[test]
    fn rejects_fewer_than_two_knots() {
        assert_eq!(
            Curve::new(vec![]),
            Err(CurveError::TooFewKnots { count: 0 })
        );
        assert_eq!(
            Curve::new(vec![Knot::new(0.0, 0.0)]),
            Err(CurveError::TooFewKnots { count: 1 })
        );
    }

    #[test]
    fn rejects_equal_x() {
        let err = Curve::new(vec![Knot::new(0.0, 0.0), Knot::new(0.0, 1.0)]).unwrap_err();
        assert_eq!(
            err,
            CurveError::NonIncreasingX {
                index: 1,
                prev_x: 0.0,
                x: 0.0
            }
        );
    }

    #[test]
    fn rejects_decreasing_x() {
        let err = Curve::new(vec![
            Knot::new(0.0, 0.0),
            Knot::new(2.0, 1.0),
            Knot::new(1.0, 1.0),
        ])
        .unwrap_err();
        assert_eq!(
            err,
            CurveError::NonIncreasingX {
                index: 2,
                prev_x: 2.0,
                x: 1.0
            }
        );
    }

    #[test]
    fn rejects_non_finite_coordinate() {
        let err = Curve::new(vec![Knot::new(0.0, 0.0), Knot::new(f64::INFINITY, 1.0)]).unwrap_err();
        match err {
            CurveError::NonFiniteCoordinate { index, x, y } => {
                assert_eq!(index, 1);
                assert!(x.is_infinite());
                assert_eq!(y, 1.0);
            }
            other => panic!("expected NonFiniteCoordinate, got {other:?}"),
        }
    }

    #[test]
    fn preserves_user_set_tangents() {
        let curve = Curve::new(vec![Knot::new(0.0, 0.0), Knot::with_tangent(1.0, 1.0, 0.5)])
            .expect("valid knots");
        assert_eq!(curve.knots()[1].tangent, Some(0.5));
    }

    #[test]
    fn rejects_non_finite_user_tangent() {
        let err = Curve::new(vec![
            Knot::new(0.0, 0.0),
            Knot::with_tangent(1.0, 1.0, f64::NAN),
        ])
        .unwrap_err();
        match err {
            CurveError::NonFiniteTangent { index, tangent } => {
                assert_eq!(index, 1);
                assert!(tangent.is_nan());
            }
            other => panic!("expected NonFiniteTangent, got {other:?}"),
        }
    }

    #[test]
    fn extend_joins_c1_at_the_resume_point() {
        let base = Curve::new(vec![
            Knot::new(0.0, 0.0),
            Knot::new(1.0, 1.0),
            Knot::new(2.0, 0.0),
        ])
        .unwrap();
        let end_slope = base.fit().end_slope();
        let joined = base.extend(vec![Knot::new(3.0, 2.0)]).unwrap();

        // The first new segment (starting at x=2) must begin on the previous
        // stroke's ending slope, so the join is C¹ with no kink.
        let spline = joined.fit();
        let new_segment = &spline.segments()[2];
        assert!((new_segment.coeffs[1] - end_slope).abs() < 1e-9);
    }

    #[test]
    fn extend_widens_the_domain() {
        let base = Curve::new(vec![Knot::new(0.0, 0.0), Knot::new(1.0, 1.0)]).unwrap();
        let joined = base
            .extend(vec![Knot::new(2.0, 2.0), Knot::new(3.0, 1.0)])
            .unwrap();
        assert_eq!(joined.domain(), (0.0, 3.0));
    }

    #[test]
    fn extend_rejects_backward_knots() {
        let base = Curve::new(vec![Knot::new(0.0, 0.0), Knot::new(2.0, 1.0)]).unwrap();
        assert!(base.extend(vec![Knot::new(1.0, 0.0)]).is_err());
    }
}
