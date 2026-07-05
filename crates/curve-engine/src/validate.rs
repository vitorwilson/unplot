//! Pure hard-block validators. The drawing UI consults these at pen-capture and
//! edit time so an invalid curve is refused *by construction* (docs/PLAN.md:
//! "Hard-block, not soft-correct"). They live in the core so the UI never has to
//! re-implement — or drift from — the rules the engine enforces.

use crate::knot::Knot;

/// Whether `next_x` may follow `prev_x` while drawing left to right: x must
/// strictly increase, keeping the curve single-valued.
///
/// # Example
/// ```
/// use curve_engine::advances_in_x;
/// assert!(advances_in_x(0.0, 0.1));
/// assert!(!advances_in_x(1.0, 1.0));
/// ```
pub fn advances_in_x(prev_x: f64, next_x: f64) -> bool {
    next_x > prev_x
}

/// Whether the step from `(prev_x, prev_y)` to `(next_x, next_y)` stays at or
/// below `max_abs_slope`. The cap blocks near-vertical spikes — a sharp corner
/// is a very large `|dy/dx|`. Returns `false` if x does not advance (so a caller
/// can use this as the single gate on a candidate sample).
pub fn within_slope_cap(
    prev_x: f64,
    prev_y: f64,
    next_x: f64,
    next_y: f64,
    max_abs_slope: f64,
) -> bool {
    if next_x <= prev_x {
        return false;
    }
    ((next_y - prev_y) / (next_x - prev_x)).abs() <= max_abs_slope
}

/// Whether moving the knot at `index` to `new_x` keeps x strictly increasing.
/// The moved knot must stay strictly between its neighbours; an endpoint only
/// has to clear its single inner neighbour. Out-of-range `index` returns `false`.
pub fn edit_keeps_order(knots: &[Knot], index: usize, new_x: f64) -> bool {
    if index >= knots.len() {
        return false;
    }
    let clears_left = index == 0 || knots[index - 1].x < new_x;
    let clears_right = index + 1 >= knots.len() || new_x < knots[index + 1].x;
    clears_left && clears_right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advances_only_when_x_increases() {
        assert!(advances_in_x(0.0, 0.001));
        assert!(!advances_in_x(1.0, 1.0));
        assert!(!advances_in_x(1.0, 0.5));
    }

    #[test]
    fn slope_cap_blocks_spikes() {
        assert!(within_slope_cap(0.0, 0.0, 1.0, 2.0, 5.0));
        assert!(!within_slope_cap(0.0, 0.0, 0.1, 10.0, 5.0));
        assert!(within_slope_cap(0.0, 0.0, 1.0, 5.0, 5.0)); // exactly at the cap
        assert!(!within_slope_cap(0.0, 0.0, 1.0, -9.0, 5.0)); // steep downward too
    }

    #[test]
    fn slope_cap_rejects_non_advancing_x() {
        assert!(!within_slope_cap(1.0, 0.0, 1.0, 0.0, 5.0));
        assert!(!within_slope_cap(1.0, 0.0, 0.5, 0.0, 5.0));
    }

    fn row() -> Vec<Knot> {
        vec![
            Knot::new(0.0, 0.0),
            Knot::new(1.0, 0.0),
            Knot::new(2.0, 0.0),
            Knot::new(3.0, 0.0),
        ]
    }

    #[test]
    fn edit_stays_between_neighbours() {
        let knots = row();
        assert!(edit_keeps_order(&knots, 1, 1.5));
        assert!(!edit_keeps_order(&knots, 1, 2.0)); // onto right neighbour
        assert!(!edit_keeps_order(&knots, 1, 0.0)); // onto left neighbour
        assert!(!edit_keeps_order(&knots, 2, 0.5)); // past left neighbour
    }

    #[test]
    fn edit_endpoints_need_one_side_only() {
        let knots = row();
        assert!(edit_keeps_order(&knots, 0, -5.0)); // first knot slides left freely
        assert!(!edit_keeps_order(&knots, 0, 1.0)); // but not onto its right neighbour
        assert!(edit_keeps_order(&knots, 3, 99.0)); // last knot slides right freely
        assert!(!edit_keeps_order(&knots, 3, 2.0)); // but not onto its left neighbour
    }

    #[test]
    fn edit_rejects_out_of_range_index() {
        assert!(!edit_keeps_order(&row(), 9, 1.5));
    }
}
