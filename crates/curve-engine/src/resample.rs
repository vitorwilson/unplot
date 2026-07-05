//! Thin dense, noisy pen samples into a minimal set of knots.
//!
//! Hundreds of non-uniform mouse samples would over-fit the spline to jitter, so
//! we simplify with Ramer–Douglas–Peucker: points are dropped where the stroke
//! is nearly straight and kept where it bends. This is curvature-aware thinning
//! (docs/PLAN.md, Phase 1) with no external dependency.

use crate::knot::Knot;

/// Simplify `samples` to a minimal knot set whose spline stays within
/// `tolerance` of the original stroke.
///
/// `tolerance` is the maximum allowed perpendicular deviation, in the samples'
/// own units — larger drops more points. Samples are assumed ordered with
/// strictly increasing x (the capture hard-block guarantees this); endpoints are
/// always kept, and fewer than three samples pass through unchanged.
///
/// # Example
/// ```
/// use curve_engine::resample;
/// let flat = [(0.0, 0.0), (0.5, 0.0), (1.0, 0.0)]; // redundant middle point
/// assert_eq!(resample(&flat, 0.01).len(), 2);
/// ```
pub fn resample(samples: &[(f64, f64)], tolerance: f64) -> Vec<Knot> {
    if samples.len() < 3 {
        return samples.iter().map(|&(x, y)| Knot::new(x, y)).collect();
    }
    let mut keep = vec![false; samples.len()];
    keep[0] = true;
    keep[samples.len() - 1] = true;
    mark_significant(samples, tolerance, &mut keep);
    samples
        .iter()
        .zip(keep)
        .filter_map(|(&(x, y), kept)| kept.then_some(Knot::new(x, y)))
        .collect()
}

/// Ramer–Douglas–Peucker, iterative (an explicit stack avoids recursion-depth
/// risk on long strokes): on each span, keep the point of maximum deviation if it
/// exceeds `tolerance`, then process the two halves it creates.
fn mark_significant(samples: &[(f64, f64)], tolerance: f64, keep: &mut [bool]) {
    let mut spans = vec![(0usize, samples.len() - 1)];
    while let Some((lo, hi)) = spans.pop() {
        if hi <= lo + 1 {
            continue;
        }
        let (index, deviation) = farthest_from_chord(samples, lo, hi);
        if deviation > tolerance {
            keep[index] = true;
            spans.push((lo, index));
            spans.push((index, hi));
        }
    }
}

/// Index and perpendicular distance of the interior sample farthest from the
/// chord `samples[lo]..samples[hi]`.
fn farthest_from_chord(samples: &[(f64, f64)], lo: usize, hi: usize) -> (usize, f64) {
    let (ax, ay) = samples[lo];
    let (bx, by) = samples[hi];
    let dx = bx - ax;
    let dy = by - ay;
    let chord = (dx * dx + dy * dy).sqrt();

    let mut best_index = lo;
    let mut best_deviation = 0.0;
    for (offset, &(px, py)) in samples[lo + 1..hi].iter().enumerate() {
        let deviation = if chord > 0.0 {
            // Distance from p to the line a-b via the 2-D cross product.
            ((px - ax) * dy - (py - ay) * dx).abs() / chord
        } else {
            ((px - ax).powi(2) + (py - ay).powi(2)).sqrt()
        };
        if deviation > best_deviation {
            best_deviation = deviation;
            best_index = lo + 1 + offset;
        }
    }
    (best_index, best_deviation)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Curve;

    #[test]
    fn passes_through_short_inputs() {
        assert_eq!(resample(&[(0.0, 0.0), (1.0, 1.0)], 0.5).len(), 2);
        assert_eq!(resample(&[(0.0, 0.0)], 0.5).len(), 1);
    }

    #[test]
    fn collapses_collinear_samples_to_endpoints() {
        let samples: Vec<(f64, f64)> = (0..=10).map(|i| (i as f64 * 0.1, i as f64 * 0.2)).collect();
        let knots = resample(&samples, 1e-6);
        assert_eq!(knots.len(), 2);
        assert_eq!(knots[0], Knot::new(samples[0].0, samples[0].1));
        assert_eq!(knots[1], Knot::new(samples[10].0, samples[10].1));
    }

    #[test]
    fn keeps_a_pronounced_corner() {
        // A tent: rise to an apex, then fall. The apex must survive.
        let samples = vec![(0.0, 0.0), (0.5, 0.5), (1.0, 1.0), (1.5, 0.5), (2.0, 0.0)];
        let knots = resample(&samples, 0.1);
        assert!(knots.len() >= 3);
        assert!(
            knots
                .iter()
                .any(|k| (k.x - 1.0).abs() < 1e-9 && (k.y - 1.0).abs() < 1e-9),
            "apex was dropped: {knots:?}"
        );
    }

    #[test]
    fn larger_tolerance_keeps_fewer_knots() {
        let samples: Vec<(f64, f64)> = (0..=20)
            .map(|i| {
                let x = i as f64 * 0.1;
                (x, (x * 3.0).sin())
            })
            .collect();
        let fine = resample(&samples, 0.01);
        let coarse = resample(&samples, 0.5);
        assert!(coarse.len() <= fine.len());
        assert!(coarse.len() >= 2);
    }

    #[test]
    fn resampled_knots_build_a_valid_curve() {
        let samples: Vec<(f64, f64)> = (0..=30)
            .map(|i| {
                let x = i as f64 * 0.1;
                (x, x.cos())
            })
            .collect();
        let knots = resample(&samples, 0.02);
        assert!(Curve::new(knots).is_ok());
    }
}
