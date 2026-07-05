//! Property tests for the curve engine's Phase 1 "Done when" criteria: strictly
//! increasing x is impossible to violate, fitted splines are C¹ across every
//! join, and evaluation interpolates the knots — checked over randomised inputs.

use curve_engine::{Curve, Knot};
use proptest::prelude::*;

/// 2..12 knots with strictly increasing x (built from positive gaps so the
/// invariant always holds) and bounded y.
fn strictly_increasing_knots() -> impl Strategy<Value = Vec<Knot>> {
    prop::collection::vec((0.01f64..10.0, -100.0f64..100.0), 2..12).prop_map(|steps| {
        let mut x = 0.0;
        steps
            .into_iter()
            .map(|(gap, y)| {
                x += gap;
                Knot::new(x, y)
            })
            .collect()
    })
}

proptest! {
    /// A curve is accepted exactly when its x values strictly increase.
    #[test]
    fn accepts_iff_x_strictly_increases(xs in prop::collection::vec(-50.0f64..50.0, 2..12)) {
        let knots: Vec<Knot> = xs.iter().map(|&x| Knot::new(x, 0.0)).collect();
        let strictly_increasing = xs.windows(2).all(|w| w[1] > w[0]);
        prop_assert_eq!(Curve::new(knots).is_ok(), strictly_increasing);
    }

    /// The left segment's ending slope equals the right segment's starting slope
    /// at every interior join — C¹ everywhere.
    #[test]
    fn fitted_spline_is_c1_across_every_join(knots in strictly_increasing_knots()) {
        let spline = Curve::new(knots).unwrap().fit();
        for pair in spline.segments().windows(2) {
            let left = pair[0];
            let h = left.x_end - left.x_start;
            let [_, b, c, d] = left.coeffs;
            let left_end_slope = b + 2.0 * c * h + 3.0 * d * h * h;
            let right_start_slope = pair[1].coeffs[1];
            prop_assert!(
                (left_end_slope - right_start_slope).abs()
                    <= 1e-6 * (1.0 + right_start_slope.abs())
            );
        }
    }

    /// The fitted spline passes through every knot.
    #[test]
    fn eval_interpolates_every_knot(knots in strictly_increasing_knots()) {
        let curve = Curve::new(knots).unwrap();
        let spline = curve.fit();
        for knot in curve.knots() {
            prop_assert!((spline.eval(knot.x) - knot.y).abs() <= 1e-6 * (1.0 + knot.y.abs()));
        }
    }
}
