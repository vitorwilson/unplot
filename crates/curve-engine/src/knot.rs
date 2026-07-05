/// A point the curve passes through.
///
/// `tangent` is the slope dy/dx the curve should have at this knot: `None` means
/// "let the fitter choose it" (Fritsch–Carlson conditioning), while `Some(m)` is
/// a user override — the drag-the-tangent-handle interaction (see docs/PLAN.md).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Knot {
    pub x: f64,
    pub y: f64,
    pub tangent: Option<f64>,
}

impl Knot {
    /// A knot whose slope will be chosen by the fitter.
    ///
    /// # Example
    /// ```
    /// use curve_engine::Knot;
    /// let k = Knot::new(0.0, 1.0);
    /// assert_eq!(k.tangent, None);
    /// ```
    pub fn new(x: f64, y: f64) -> Self {
        Knot {
            x,
            y,
            tangent: None,
        }
    }

    /// A knot whose slope is pinned to `tangent` (a user override).
    ///
    /// # Example
    /// ```
    /// use curve_engine::Knot;
    /// let k = Knot::with_tangent(0.0, 1.0, 2.0);
    /// assert_eq!(k.tangent, Some(2.0));
    /// ```
    pub fn with_tangent(x: f64, y: f64, tangent: f64) -> Self {
        Knot {
            x,
            y,
            tangent: Some(tangent),
        }
    }
}
