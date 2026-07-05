use std::fmt;

/// Why a set of knots cannot form a valid curve.
///
/// Every variant carries the offending value(s) so the caller — and the UI's
/// hard-block at capture time — can explain precisely why the input was refused
/// (see docs/PLAN.md, Phase 1).
#[derive(Debug, Clone, PartialEq)]
pub enum CurveError {
    /// A curve needs at least two knots to span an interval.
    TooFewKnots { count: usize },
    /// x must be strictly increasing; the knot at `index` had x not greater
    /// than the previous knot's x.
    NonIncreasingX { index: usize, prev_x: f64, x: f64 },
    /// A coordinate was NaN or infinite.
    NonFiniteCoordinate { index: usize, x: f64, y: f64 },
    /// A user-set tangent (slope) was NaN or infinite.
    NonFiniteTangent { index: usize, tangent: f64 },
}

impl fmt::Display for CurveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CurveError::TooFewKnots { count } => {
                write!(f, "curve needs at least 2 knots, got {count}")
            }
            CurveError::NonIncreasingX { index, prev_x, x } => write!(
                f,
                "x must strictly increase: knot {index} has x={x}, not greater than previous x={prev_x}"
            ),
            CurveError::NonFiniteCoordinate { index, x, y } => {
                write!(f, "knot {index} has a non-finite coordinate: ({x}, {y})")
            }
            CurveError::NonFiniteTangent { index, tangent } => {
                write!(f, "knot {index} has a non-finite tangent: {tangent}")
            }
        }
    }
}

impl std::error::Error for CurveError {}

#[cfg(test)]
mod tests {
    use super::CurveError;

    #[test]
    fn messages_name_the_offending_values() {
        let too_few = CurveError::TooFewKnots { count: 1 }.to_string();
        assert!(too_few.contains('1'), "message was: {too_few}");

        let increasing = CurveError::NonIncreasingX {
            index: 2,
            prev_x: 3.0,
            x: 1.0,
        }
        .to_string();
        assert!(increasing.contains("x=1"), "message was: {increasing}");
        assert!(increasing.contains("x=3"), "message was: {increasing}");
    }
}
