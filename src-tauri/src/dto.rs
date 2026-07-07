use curve_engine::{Curve, Knot};
use serde::{Deserialize, Serialize};

/// A knot as exchanged with the frontend: position, an optional user-set tangent
/// (`None` = fitter chooses, `Some` = a dragged tangent handle), and the
/// effective `slope` in the fitted curve. `slope` is output-only — it is ignored
/// on input (the fitter recomputes it) and drives handle rendering on output.
#[derive(Serialize, Deserialize, Clone, Copy)]
pub(crate) struct KnotDto {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) tangent: Option<f64>,
    #[serde(default)]
    pub(crate) slope: f64,
}

impl KnotDto {
    fn to_knot(self) -> Knot {
        match self.tangent {
            Some(slope) => Knot::with_tangent(self.x, self.y, slope),
            None => Knot::new(self.x, self.y),
        }
    }
}

/// The result of fitting a curve: its knots (for editing/resume) and a dense
/// polyline of the smooth spline, both in world coordinates.
#[derive(Serialize)]
pub(crate) struct FittedCurve {
    pub(crate) knots: Vec<KnotDto>,
    pub(crate) polyline: Vec<[f64; 2]>,
}

/// How many points to sample along the fitted spline for rendering.
pub(crate) const POLYLINE_POINTS: usize = 256;

/// The exact function in every copy target: a one-line summary (shown collapsed),
/// the KaTeX `cases` block (shown on expand and copied as raw LaTeX), and the
/// Desmos / Wolfram paste forms. All are derived from one fit so a format switch
/// in the UI needs no extra round-trip. Pressing "Done" calls this.
#[derive(Serialize)]
pub(crate) struct CurveLatex {
    pub(crate) summary: String,
    pub(crate) latex: String,
    pub(crate) desmos: String,
    pub(crate) wolfram: String,
    /// A compact closed form offered only when the fit is trustworthy (Phase 7);
    /// `None` means the exact output stands alone.
    pub(crate) approximation: Option<Approximation>,
}

/// The "prettier function": a closed-form LaTeX approximation plus its error as a
/// fraction of the curve's y-range (so the UI can show "max 0.4%").
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Approximation {
    pub(crate) latex: String,
    pub(crate) max_error: f64,
    pub(crate) rms_error: f64,
}

/// A calculus operation the UI chains onto the drawn curve, arriving from the
/// frontend as `"differentiate"` / `"integrate"`.
#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum CalcOp {
    Differentiate,
    Integrate,
}

/// A calculus result for display: the transformed curve's polyline (to draw on
/// the plane) plus its math in every copy format. The original knots stay the
/// source of truth and `ops` is replayed on every click, so chaining never has
/// to ship an intermediate (general-degree) curve back and forth.
#[derive(Serialize)]
pub(crate) struct CalcCurve {
    pub(crate) polyline: Vec<[f64; 2]>,
    pub(crate) summary: String,
    pub(crate) latex: String,
    pub(crate) desmos: String,
    pub(crate) wolfram: String,
    /// A closed form for the derivative/integral itself (e.g. d/dx of a parabola
    /// is `2x`), offered on the same terms as for the drawn curve.
    pub(crate) approximation: Option<Approximation>,
    /// `true` when the result is the exact symbolic derivative/integral of a
    /// recognized function (a clean, smooth closed form), `false` for the numeric
    /// piecewise result — so the UI can label it honestly (no "corners" note).
    pub(crate) exact: bool,
}

pub(crate) fn pairs(samples: &[[f64; 2]]) -> Vec<(f64, f64)> {
    samples.iter().map(|&[x, y]| (x, y)).collect()
}

pub(crate) fn to_knots(dtos: &[KnotDto]) -> Vec<Knot> {
    dtos.iter().map(|dto| dto.to_knot()).collect()
}

/// Serialize a fitted curve for the frontend: its knots (positions + any user
/// tangents, for editing/resume) and a dense polyline of its smooth spline.
pub(crate) fn render(curve: &Curve) -> FittedCurve {
    let spline = curve.fit();
    let slopes = spline.knot_slopes();
    FittedCurve {
        knots: curve
            .knots()
            .iter()
            .zip(slopes)
            .map(|(knot, slope)| KnotDto {
                x: knot.x,
                y: knot.y,
                tangent: knot.tangent,
                slope,
            })
            .collect(),
        polyline: spline
            .polyline(POLYLINE_POINTS)
            .iter()
            .map(|&(x, y)| [x, y])
            .collect(),
    }
}

/// A `KnotDto` built for tests, with the output-only `slope` zeroed. Shared by the
/// command test modules via `use crate::dto::dto`.
#[cfg(test)]
pub(crate) fn dto(x: f64, y: f64, tangent: Option<f64>) -> KnotDto {
    KnotDto {
        x,
        y,
        tangent,
        slope: 0.0,
    }
}
