use crate::knot::Knot;

/// One polynomial piece of a piecewise curve, in power basis about its left
/// endpoint: `y(x) = Σ coeffs[k]·tᵏ` where `t = x − x_start`.
///
/// A fitted spline's pieces are cubic (`coeffs = [a, b, c, d]`). The power basis
/// (rather than the Hermite form) is stored because calculus and LaTeX both work
/// directly from the coefficients, with no CAS: differentiating a piece lowers
/// its degree and integrating raises it (Phase 5), so `coeffs` is a `Vec` rather
/// than a fixed `[f64; 4]` — a quartic antiderivative (or a chain of them) simply
/// carries more terms.
#[derive(Debug, Clone, PartialEq)]
pub struct Segment {
    pub x_start: f64,
    pub x_end: f64,
    pub coeffs: Vec<f64>,
}

impl Segment {
    /// Build the segment for the Hermite piece between two knots with the given
    /// endpoint slopes, converting to power-basis coefficients about `x_start`.
    fn hermite(start: &Knot, end: &Knot, m_start: f64, m_end: f64) -> Segment {
        let h = end.x - start.x;
        let dy = end.y - start.y;
        let a = start.y;
        let b = m_start;
        let c = (3.0 * dy - h * (2.0 * m_start + m_end)) / (h * h);
        let d = (h * (m_start + m_end) - 2.0 * dy) / (h * h * h);
        Segment {
            x_start: start.x,
            x_end: end.x,
            coeffs: vec![a, b, c, d],
        }
    }

    /// Evaluate the polynomial at `x` (assumed within `[x_start, x_end]`) via
    /// Horner, for any degree.
    fn eval(&self, x: f64) -> f64 {
        let t = x - self.x_start;
        self.coeffs.iter().rev().fold(0.0, |acc, &c| acc * t + c)
    }

    /// The slope `y'(x_start + t)` at local offset `t`, i.e. `Σ_{k≥1} k·coeffs[k]·
    /// tᵏ⁻¹`, for any degree. On a fitted cubic this is `b + 2c·t + 3d·t²`.
    fn slope_at(&self, t: f64) -> f64 {
        self.coeffs
            .iter()
            .enumerate()
            .skip(1)
            .fold(0.0, |acc, (k, &c)| {
                acc + (k as f64) * c * t.powi(k as i32 - 1)
            })
    }
}

/// A fitted, shape-preserving piecewise cubic Hermite spline over a closed
/// domain. Produced by [`crate::Curve::fit`]; evaluation clamps to the domain
/// (no extrapolation — a Locked decision in docs/PLAN.md).
#[derive(Debug, Clone, PartialEq)]
pub struct Spline {
    segments: Vec<Segment>,
    domain: (f64, f64),
}

impl Spline {
    /// Evaluate `f(x)`. Outside `[a, b]` the result is clamped to the nearest
    /// endpoint value; the curve is never extended past what was drawn.
    ///
    /// # Example
    /// ```
    /// use curve_engine::{Curve, Knot};
    /// let spline = Curve::new(vec![Knot::new(0.0, 0.0), Knot::new(2.0, 4.0)])
    ///     .unwrap()
    ///     .fit();
    /// assert!((spline.eval(1.0) - 2.0).abs() < 1e-9); // the straight line y = 2x
    /// ```
    pub fn eval(&self, x: f64) -> f64 {
        let (a, b) = self.domain;
        let clamped = x.clamp(a, b);
        self.segment_at(clamped).eval(clamped)
    }

    /// The closed interval `[x_first, x_last]` the spline is defined on.
    pub fn domain(&self) -> (f64, f64) {
        self.domain
    }

    /// The polynomial pieces, left to right.
    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }

    /// Slope at the left end of the domain, `f'(x_first)`.
    pub fn start_slope(&self) -> f64 {
        self.segments[0].slope_at(0.0)
    }

    /// Slope at the right end of the domain, `f'(x_last)`. Used when resuming a
    /// drawing so the next stroke can join C¹.
    pub fn end_slope(&self) -> f64 {
        let last = &self.segments[self.segments.len() - 1];
        last.slope_at(last.x_end - last.x_start)
    }

    /// Slope at each knot, left to right (one per knot). The interior/left values
    /// are each segment's starting slope; the last is the domain's end slope.
    /// C¹ continuity means these are the unambiguous slopes at the joins — used to
    /// render the draggable tangent handles.
    pub fn knot_slopes(&self) -> Vec<f64> {
        let mut slopes: Vec<f64> = self.segments.iter().map(|s| s.slope_at(0.0)).collect();
        slopes.push(self.end_slope());
        slopes
    }

    /// Sample the spline at `count` evenly spaced x across the domain, for
    /// rendering the smooth curve. `count` is clamped to at least 2.
    pub fn polyline(&self, count: usize) -> Vec<(f64, f64)> {
        let n = count.max(2);
        let (a, b) = self.domain;
        (0..n)
            .map(|i| {
                let t = i as f64 / (n - 1) as f64;
                let x = a + t * (b - a);
                (x, self.eval(x))
            })
            .collect()
    }

    /// The segment whose half-open span contains `x` (the last one starting at
    /// or before `x`). `x` is assumed already clamped to the domain.
    fn segment_at(&self, x: f64) -> &Segment {
        let count = self.segments.partition_point(|s| s.x_start <= x);
        let idx = count.saturating_sub(1).min(self.segments.len() - 1);
        &self.segments[idx]
    }

    /// Build a spline directly from ready-made polynomial pieces — used by
    /// calculus, where pieces come from differentiating or integrating an
    /// existing curve rather than from fitting knots. `pieces` must be non-empty
    /// and left-to-right contiguous; the domain is taken from the outer edges.
    pub(crate) fn from_pieces(pieces: Vec<Segment>) -> Spline {
        let domain = (pieces[0].x_start, pieces[pieces.len() - 1].x_end);
        Spline {
            segments: pieces,
            domain,
        }
    }
}

/// Fit the spline for a validated, strictly-increasing set of knots.
pub(crate) fn fit_spline(knots: &[Knot]) -> Spline {
    let tangents = pchip_tangents(knots);
    let segments = knots
        .windows(2)
        .zip(tangents.windows(2))
        .map(|(pair, m)| Segment::hermite(&pair[0], &pair[1], m[0], m[1]))
        .collect();
    let domain = (knots[0].x, knots[knots.len() - 1].x);
    Spline { segments, domain }
}

/// Fritsch–Carlson (PCHIP) tangents: node slopes that make the Hermite spline
/// monotone where the data is monotone and free of overshoot. A user-set tangent
/// overrides the computed value at that knot (the drag-the-slope interaction).
fn pchip_tangents(knots: &[Knot]) -> Vec<f64> {
    let n = knots.len();
    let h: Vec<f64> = knots.windows(2).map(|p| p[1].x - p[0].x).collect();
    let secant: Vec<f64> = knots
        .windows(2)
        .zip(&h)
        .map(|(p, &hk)| (p[1].y - p[0].y) / hk)
        .collect();

    let mut m = vec![0.0; n];
    for k in 1..n - 1 {
        m[k] = pchip_interior(h[k - 1], h[k], secant[k - 1], secant[k]);
    }
    if n == 2 {
        m[0] = secant[0];
        m[1] = secant[0];
    } else {
        m[0] = pchip_endpoint(h[0], h[1], secant[0], secant[1]);
        m[n - 1] = pchip_endpoint(h[n - 2], h[n - 3], secant[n - 2], secant[n - 3]);
    }

    for (slope, knot) in m.iter_mut().zip(knots) {
        if let Some(user) = knot.tangent {
            *slope = user;
        }
    }
    m
}

/// Interior slope: the weighted harmonic mean of neighbouring secants, or zero
/// at a local extremum (opposite-signed or flat secants) to prevent overshoot.
fn pchip_interior(h_prev: f64, h_next: f64, d_prev: f64, d_next: f64) -> f64 {
    if d_prev * d_next <= 0.0 {
        return 0.0;
    }
    let w1 = 2.0 * h_next + h_prev;
    let w2 = h_next + 2.0 * h_prev;
    (w1 + w2) / (w1 / d_prev + w2 / d_next)
}

/// Endpoint slope: a non-centred three-point estimate, limited so it cannot
/// introduce an overshoot (Fritsch–Carlson end conditions, as in SciPy's PCHIP).
fn pchip_endpoint(h_near: f64, h_far: f64, d_near: f64, d_far: f64) -> f64 {
    let slope = ((2.0 * h_near + h_far) * d_near - h_near * d_far) / (h_near + h_far);
    if slope * d_near <= 0.0 {
        0.0
    } else if d_near * d_far < 0.0 && slope.abs() > 3.0 * d_near.abs() {
        3.0 * d_near
    } else {
        slope
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fit(knots: Vec<Knot>) -> Spline {
        fit_spline(&knots)
    }

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-9,
            "expected {actual} ≈ {expected}"
        );
    }

    #[test]
    fn reports_boundary_slopes_of_a_line() {
        let spline = fit(vec![Knot::new(0.0, 0.0), Knot::new(2.0, 4.0)]);
        assert_close(spline.start_slope(), 2.0);
        assert_close(spline.end_slope(), 2.0);
    }

    #[test]
    fn knot_slopes_of_a_line_are_all_the_gradient() {
        let spline = fit(vec![
            Knot::new(0.0, 0.0),
            Knot::new(1.0, 2.0),
            Knot::new(2.0, 4.0),
        ]);
        let slopes = spline.knot_slopes();
        assert_eq!(slopes.len(), 3);
        for slope in slopes {
            assert_close(slope, 2.0);
        }
    }

    #[test]
    fn polyline_spans_the_domain() {
        let spline = fit(vec![Knot::new(0.0, 0.0), Knot::new(2.0, 4.0)]);
        let pts = spline.polyline(5);
        assert_eq!(pts.len(), 5);
        assert_close(pts[0].0, 0.0);
        assert_close(pts[0].1, 0.0);
        assert_close(pts[4].0, 2.0);
        assert_close(pts[4].1, 4.0);
        assert_close(pts[2].1, 2.0); // y = 2x at the midpoint x = 1
    }

    #[test]
    fn reproduces_a_straight_line() {
        // Collinear knots -> every tangent equals the common slope -> exact line.
        let spline = fit(vec![
            Knot::new(0.0, 0.0),
            Knot::new(1.0, 2.0),
            Knot::new(2.0, 4.0),
        ]);
        assert_close(spline.eval(0.5), 1.0);
        assert_close(spline.eval(1.5), 3.0);
    }

    #[test]
    fn interpolates_every_knot() {
        let knots = vec![
            Knot::new(0.0, 0.0),
            Knot::new(1.0, 3.0),
            Knot::new(2.5, -2.0),
            Knot::new(4.0, 1.0),
        ];
        let spline = fit(knots.clone());
        for knot in &knots {
            assert_close(spline.eval(knot.x), knot.y);
        }
    }

    #[test]
    fn clamps_outside_the_domain() {
        let spline = fit(vec![Knot::new(0.0, 1.0), Knot::new(2.0, 5.0)]);
        assert_close(spline.eval(-10.0), 1.0);
        assert_close(spline.eval(0.0), 1.0);
        assert_close(spline.eval(2.0), 5.0);
        assert_close(spline.eval(99.0), 5.0);
    }

    #[test]
    fn smoothstep_from_user_tangents() {
        // Flat tangents at both ends over [0, 1] give exactly 3t² − 2t³.
        let spline = fit(vec![
            Knot::with_tangent(0.0, 0.0, 0.0),
            Knot::with_tangent(1.0, 1.0, 0.0),
        ]);
        assert_close(spline.eval(0.5), 0.5);
        assert_close(spline.eval(0.25), 3.0 * 0.0625 - 2.0 * 0.015625);
    }

    #[test]
    fn c1_continuous_across_joins() {
        let spline = fit(vec![
            Knot::new(0.0, 0.0),
            Knot::new(1.0, 2.0),
            Knot::new(2.0, 1.0),
            Knot::new(3.0, 4.0),
        ]);
        for pair in spline.segments().windows(2) {
            let left = &pair[0];
            let slope_left_end = left.slope_at(left.x_end - left.x_start);
            let slope_right_start = pair[1].slope_at(0.0);
            assert_close(slope_left_end, slope_right_start);
        }
    }

    #[test]
    fn monotone_data_stays_monotone() {
        // Shape preservation: rising knots must never dip between them.
        let spline = fit(vec![
            Knot::new(0.0, 0.0),
            Knot::new(1.0, 1.0),
            Knot::new(2.0, 1.2),
            Knot::new(3.0, 10.0),
        ]);
        let mut prev = spline.eval(0.0);
        let mut x = 0.0;
        while x <= 3.0 {
            let y = spline.eval(x);
            assert!(y >= prev - 1e-9, "dip at x={x}: {y} < {prev}");
            prev = y;
            x += 0.01;
        }
    }
}
