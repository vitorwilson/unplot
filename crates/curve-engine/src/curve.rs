use crate::error::CurveError;
use crate::knot::Knot;

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
}
