use curve_engine::symbolic::Expr;
use curve_engine::{Curve, Knot};
use serde::{Deserialize, Serialize};

/// A knot as exchanged with the frontend: position, an optional user-set tangent
/// (`None` = fitter chooses, `Some` = a dragged tangent handle), and the
/// effective `slope` in the fitted curve. `slope` is output-only — it is ignored
/// on input (the fitter recomputes it) and drives handle rendering on output.
#[derive(Serialize, Deserialize, Clone, Copy)]
struct KnotDto {
    x: f64,
    y: f64,
    tangent: Option<f64>,
    #[serde(default)]
    slope: f64,
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
struct FittedCurve {
    knots: Vec<KnotDto>,
    polyline: Vec<[f64; 2]>,
}

/// How many points to sample along the fitted spline for rendering.
const POLYLINE_POINTS: usize = 256;

/// Reports the headless core's version — the first bridge from the UI shell into
/// the curve engine.
#[tauri::command]
fn engine_version() -> String {
    curve_engine::engine_version().to_string()
}

/// The application's version, for the About dialog. Read from the app crate at
/// compile time, so it always matches the released bundle.
#[tauri::command]
fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Resample a raw drawn stroke, fit the shape-preserving spline in the core, and
/// return it for rendering. Errors (as a message) when the stroke is not a valid
/// function — e.g. fewer than two distinct points.
#[tauri::command]
fn fit_curve(samples: Vec<[f64; 2]>, tolerance: f64) -> Result<FittedCurve, String> {
    let knots = curve_engine::resample(&pairs(&samples), tolerance);
    let curve = Curve::new(knots).map_err(|error| error.to_string())?;
    Ok(render(&curve))
}

/// Resume drawing: resample `samples` and append them to the curve described by
/// `existing` knots, joining C¹ (the lift-and-resume gesture). Errors if the new
/// stroke does not continue strictly to the right of the existing curve.
#[tauri::command]
fn extend_curve(
    existing: Vec<KnotDto>,
    samples: Vec<[f64; 2]>,
    tolerance: f64,
) -> Result<FittedCurve, String> {
    let base = Curve::new(to_knots(&existing)).map_err(|error| error.to_string())?;
    let new_knots = curve_engine::resample(&pairs(&samples), tolerance);
    let curve = base.extend(new_knots).map_err(|error| error.to_string())?;
    Ok(render(&curve))
}

/// Re-fit an edited set of knots (dragged points or tangent handles) without
/// resampling — the editing workhorse. Errors if the edit is not a valid
/// function (e.g. a knot dragged past a neighbor's x).
#[tauri::command]
fn refit_curve(knots: Vec<KnotDto>) -> Result<FittedCurve, String> {
    let curve = Curve::new(to_knots(&knots)).map_err(|error| error.to_string())?;
    Ok(render(&curve))
}

/// The exact function in every copy target: a one-line summary (shown collapsed),
/// the KaTeX `cases` block (shown on expand and copied as raw LaTeX), and the
/// Desmos / Wolfram paste forms. All are derived from one fit so a format switch
/// in the UI needs no extra round-trip. Pressing "Done" calls this.
#[derive(Serialize)]
struct CurveLatex {
    summary: String,
    latex: String,
    desmos: String,
    wolfram: String,
    /// A compact closed form offered only when the fit is trustworthy (Phase 7);
    /// `None` means the exact output stands alone.
    approximation: Option<Approximation>,
}

/// The "prettier function": a closed-form LaTeX approximation plus its error as a
/// fraction of the curve's y-range (so the UI can show "max 0.4%").
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Approximation {
    latex: String,
    max_error: f64,
    rms_error: f64,
}

#[tauri::command]
fn curve_latex(knots: Vec<KnotDto>) -> Result<CurveLatex, String> {
    let curve = Curve::new(to_knots(&knots)).map_err(|error| error.to_string())?;
    let spline = curve.fit();
    Ok(CurveLatex {
        summary: curve_engine::latex::summary(&spline),
        latex: curve_engine::latex::piecewise(&spline),
        desmos: curve_engine::export::desmos(&spline),
        wolfram: curve_engine::export::wolfram(&spline),
        // Fit the user's exact knots, not the smoothed spline: PCHIP flattens a
        // sparse wave's peaks, so sampling the spline misses a cosine the typed
        // points plainly trace (see approximate::closed_form_of_knots).
        approximation: normalize(
            curve_engine::approximate::closed_form_of_knots(curve.knots()),
            y_span(&spline),
        ),
    })
}

/// The closed-form "prettier function" for a spline (Phase 7), with error as a
/// fraction of the y-range, or `None` when no trustworthy form exists. Used for a
/// derivative/integral, whose spline is itself the exact (piecewise-polynomial)
/// result — so sampling it, rather than any knots, is the right target.
fn approximate_curve(spline: &curve_engine::Spline) -> Option<Approximation> {
    normalize(
        curve_engine::approximate::closed_form(spline),
        y_span(spline),
    )
}

/// Express a closed form's error as a fraction of the curve's y-range, so the UI
/// can show "max 0.4%". A near-flat curve floors the range to avoid dividing by
/// zero.
fn normalize(
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
fn y_span(spline: &curve_engine::Spline) -> f64 {
    let ys = spline.polyline(POLYLINE_POINTS);
    let lo = ys.iter().map(|&(_, y)| y).fold(f64::INFINITY, f64::min);
    let hi = ys.iter().map(|&(_, y)| y).fold(f64::NEG_INFINITY, f64::max);
    (hi - lo).max(0.0)
}

/// A calculus operation the UI chains onto the drawn curve, arriving from the
/// frontend as `"differentiate"` / `"integrate"`.
#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum CalcOp {
    Differentiate,
    Integrate,
}

/// A calculus result for display: the transformed curve's polyline (to draw on
/// the plane) plus its math in every copy format. The original knots stay the
/// source of truth and `ops` is replayed on every click, so chaining never has
/// to ship an intermediate (general-degree) curve back and forth.
#[derive(Serialize)]
struct CalcCurve {
    polyline: Vec<[f64; 2]>,
    summary: String,
    latex: String,
    desmos: String,
    wolfram: String,
    /// A closed form for the derivative/integral itself (e.g. d/dx of a parabola
    /// is `2x`), offered on the same terms as for the drawn curve.
    approximation: Option<Approximation>,
    /// `true` when the result is the exact symbolic derivative/integral of a
    /// recognized function (a clean, smooth closed form), `false` for the numeric
    /// piecewise result — so the UI can label it honestly (no "corners" note).
    exact: bool,
}

/// Fit `knots`, apply each calculus `op` left to right, and return the resulting
/// curve for display. When the drawn curve is recognized as a clean function, the
/// calculus is done exactly on that closed form (d/dx of x³ is 3x², not the lumpy
/// derivative of the smoothed spline); otherwise it falls back to the numeric
/// per-segment result. An empty `ops` returns the drawn curve unchanged.
#[tauri::command]
fn apply_calculus(knots: Vec<KnotDto>, ops: Vec<CalcOp>) -> Result<CalcCurve, String> {
    let curve = Curve::new(to_knots(&knots)).map_err(|error| error.to_string())?;
    let spline = curve.fit();
    Ok(symbolic_calculus(&curve, &spline, &ops).unwrap_or_else(|| numeric_calculus(&spline, &ops)))
}

/// Calculus on the recognized closed form of the drawn curve, or `None` to fall
/// back to numeric — when nothing is recognized, an op has no closed form here (a
/// rational integral), or the result is non-finite over the domain.
fn symbolic_calculus(
    curve: &Curve,
    spline: &curve_engine::Spline,
    ops: &[CalcOp],
) -> Option<CalcCurve> {
    let (start, end) = spline.domain();
    let mut expr = curve_engine::approximate::closed_form_of_knots(curve.knots())?.expr;
    for op in ops {
        expr = match op {
            CalcOp::Differentiate => expr.differentiate(),
            CalcOp::Integrate => expr.integrate(start)?,
        };
    }
    let polyline = symbolic_polyline(&expr, start, end)?;
    let form = expr.to_latex();
    Some(CalcCurve {
        polyline,
        summary: form.clone(),
        latex: format!("f(x) = {form}"),
        desmos: form,
        wolfram: expr.to_wolfram(),
        approximation: None, // the result is exact — no separate "prettier" form
        exact: true,
    })
}

/// Sample a symbolic result across `[start, end]`; `None` if it is non-finite
/// anywhere (a pole or a log's edge), so the caller falls back to numeric.
fn symbolic_polyline(expr: &Expr, start: f64, end: f64) -> Option<Vec<[f64; 2]>> {
    (0..POLYLINE_POINTS)
        .map(|i| {
            let x = start + (end - start) * i as f64 / (POLYLINE_POINTS - 1) as f64;
            let y = expr.eval(x);
            y.is_finite().then_some([x, y])
        })
        .collect()
}

/// The numeric per-segment calculus: exact on the fitted spline, and the only
/// path for a curve no closed form describes. Its derivative has corners at the
/// knots and its integral is smooth, which the UI notes (`exact: false`).
fn numeric_calculus(spline: &curve_engine::Spline, ops: &[CalcOp]) -> CalcCurve {
    let mut result = spline.clone();
    for op in ops {
        result = match op {
            CalcOp::Differentiate => curve_engine::calculus::differentiate(&result),
            CalcOp::Integrate => curve_engine::calculus::integrate(&result),
        };
    }
    CalcCurve {
        polyline: result
            .polyline(POLYLINE_POINTS)
            .iter()
            .map(|&(x, y)| [x, y])
            .collect(),
        summary: curve_engine::latex::summary(&result),
        latex: curve_engine::latex::piecewise(&result),
        desmos: curve_engine::export::desmos(&result),
        wolfram: curve_engine::export::wolfram(&result),
        approximation: approximate_curve(&result),
        exact: false,
    }
}

/// Save the drawn curve to `path` as a versioned `.unplot` document. The
/// frontend picks `path` via the native save dialog; only the knots (the source
/// of truth) are written, so the file reopens fully editable.
#[tauri::command]
fn save_curve(path: String, knots: Vec<KnotDto>) -> Result<(), String> {
    let curve = Curve::new(to_knots(&knots)).map_err(|error| error.to_string())?;
    let json = curve_engine::document::Document::from_curve(&curve).to_json();
    std::fs::write(&path, json).map_err(|error| format!("could not write {path}: {error}"))
}

/// Open a `.unplot` document from `path` and return the fitted curve for editing.
/// Errors (as a message) if the file is missing, malformed, from a newer schema,
/// or does not describe a valid function.
#[tauri::command]
fn open_curve(path: String) -> Result<FittedCurve, String> {
    let json = std::fs::read_to_string(&path)
        .map_err(|error| format!("could not read {path}: {error}"))?;
    let curve = curve_engine::document::from_json(&json)
        .map_err(|error| error.to_string())?
        .into_curve()
        .map_err(|error| error.to_string())?;
    Ok(render(&curve))
}

fn pairs(samples: &[[f64; 2]]) -> Vec<(f64, f64)> {
    samples.iter().map(|&[x, y]| (x, y)).collect()
}

fn to_knots(dtos: &[KnotDto]) -> Vec<Knot> {
    dtos.iter().map(|dto| dto.to_knot()).collect()
}

/// Serialize a fitted curve for the frontend: its knots (positions + any user
/// tangents, for editing/resume) and a dense polyline of its smooth spline.
fn render(curve: &Curve) -> FittedCurve {
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            engine_version,
            app_version,
            fit_curve,
            extend_curve,
            refit_curve,
            curve_latex,
            apply_calculus,
            save_curve,
            open_curve
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::{
        apply_calculus, curve_latex, extend_curve, fit_curve, open_curve, refit_curve, save_curve,
        CalcOp, KnotDto,
    };

    #[test]
    fn app_version_reads_the_crate_version() {
        // The About dialog shows this; it must be a real version string.
        let version = super::app_version();
        assert!(
            version.split('.').count() >= 2,
            "expected a semver-like version, got {version:?}"
        );
        assert_eq!(version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn fits_a_drawn_line() {
        let fitted = fit_curve(vec![[0.0, 0.0], [0.5, 1.0], [1.0, 2.0]], 0.05).unwrap();
        assert!(fitted.polyline.len() >= 2);
        assert!((fitted.polyline[0][0] - 0.0).abs() < 1e-9);
        assert!((fitted.polyline.last().unwrap()[0] - 1.0).abs() < 1e-9);
    }

    #[test]
    fn rejects_a_degenerate_stroke() {
        assert!(fit_curve(vec![[0.0, 0.0]], 0.05).is_err());
    }

    #[test]
    fn extends_a_curve_to_the_right() {
        let base = fit_curve(vec![[0.0, 0.0], [1.0, 1.0]], 0.05).unwrap();
        let extended = extend_curve(base.knots, vec![[2.0, 0.0], [3.0, 1.0]], 0.05).unwrap();
        // The combined curve now spans [0, 3].
        assert!((extended.polyline[0][0] - 0.0).abs() < 1e-9);
        assert!((extended.polyline.last().unwrap()[0] - 3.0).abs() < 1e-9);
    }

    #[test]
    fn rejects_a_resume_that_goes_backward() {
        let base = fit_curve(vec![[0.0, 0.0], [2.0, 1.0]], 0.05).unwrap();
        assert!(extend_curve(base.knots, vec![[1.0, 0.0], [1.5, 0.0]], 0.05).is_err());
    }

    #[test]
    fn refit_honors_a_user_tangent() {
        // A flat tangent at both ends over [0, 1] is the smoothstep 3t²−2t³,
        // which passes through 0.5 at the midpoint.
        let knots = vec![dto(0.0, 0.0, Some(0.0)), dto(1.0, 1.0, Some(0.0))];
        let fitted = refit_curve(knots).unwrap();
        let mid = fitted.polyline[fitted.polyline.len() / 2];
        assert!((mid[1] - 0.5).abs() < 0.05, "midpoint y was {}", mid[1]);
    }

    #[test]
    fn refit_reports_the_slope_at_each_knot() {
        // A straight run of knots -> every effective slope equals the gradient.
        let knots = vec![
            dto(0.0, 0.0, None),
            dto(1.0, 2.0, None),
            dto(2.0, 4.0, None),
        ];
        let fitted = refit_curve(knots).unwrap();
        for knot in fitted.knots {
            assert!((knot.slope - 2.0).abs() < 1e-9, "slope was {}", knot.slope);
        }
    }

    #[test]
    fn refit_rejects_a_knot_dragged_past_its_neighbor() {
        let knots = vec![
            dto(0.0, 0.0, None),
            dto(2.0, 1.0, None),
            dto(1.0, 1.0, None),
        ];
        assert!(refit_curve(knots).is_err());
    }

    #[test]
    fn curve_latex_returns_every_copy_format() {
        let result = curve_latex(vec![dto(0.0, 0.0, None), dto(2.0, 4.0, None)]).unwrap();
        assert_eq!(result.summary, "1-segment spline over [0, 2]");
        assert!(result.latex.contains("\\begin{cases}"));
        assert!(result.desmos.contains("\\left\\{"));
        assert!(result.wolfram.contains("Piecewise[{{"));
    }

    #[test]
    fn curve_latex_offers_a_closed_form_for_a_simple_curve() {
        // y = 2x is exactly one basis term, so a trustworthy form is offered.
        let result = curve_latex(vec![dto(0.0, 0.0, None), dto(2.0, 4.0, None)]).unwrap();
        let approx = result
            .approximation
            .expect("a line should get a closed form");
        assert!(approx.latex.contains("2x"), "{}", approx.latex);
        assert!(
            approx.max_error < 0.01,
            "relative error {}",
            approx.max_error
        );
    }

    #[test]
    fn curve_latex_recognizes_typed_cosine_points() {
        // Regression: five exact points over one period of cos(x), typed with no
        // tangents. The smoothed spline flattens the peaks, so the closed form must
        // be fitted from the knots for the app to recognize the wave.
        let quarter = std::f64::consts::PI / 2.0;
        let knots = (0..5)
            .map(|i| {
                let x = i as f64 * quarter;
                dto(x, x.cos(), None)
            })
            .collect();
        let approx = curve_latex(knots)
            .unwrap()
            .approximation
            .expect("typed cosine points should get a closed form");
        assert!(approx.latex.contains("\\cos"), "{}", approx.latex);
    }

    #[test]
    fn curve_latex_recognizes_a_typed_hyperbola() {
        // Phase 7 rational strategy: y = 1/x is a pole-shaped curve no polynomial or
        // wave can express, so the closed form should be the fraction 1/x.
        let knots = (0..8)
            .map(|i| {
                let x = 0.5 + i as f64 * 0.5;
                dto(x, 1.0 / x, None)
            })
            .collect();
        let approx = curve_latex(knots)
            .unwrap()
            .approximation
            .expect("a typed hyperbola should get a closed form");
        assert_eq!(
            approx.latex, "f(x) \\approx \\frac{1}{x}",
            "{}",
            approx.latex
        );
    }

    /// Typed points of `f` at each integer `x` in `[lo, hi]`.
    fn integer_points(lo: i32, hi: i32, f: impl Fn(f64) -> f64) -> Vec<KnotDto> {
        (lo..=hi)
            .map(|x| dto(x as f64, f(x as f64), None))
            .collect()
    }

    #[test]
    fn apply_calculus_differentiates_a_line_to_a_constant() {
        // f(x) = 2x is recognized, so its derivative is the exact constant 2 — a
        // clean symbolic result, not the numeric piecewise one.
        let result = apply_calculus(
            vec![dto(0.0, 0.0, None), dto(2.0, 4.0, None)],
            vec![CalcOp::Differentiate],
        )
        .unwrap();
        assert!(result.polyline.iter().all(|&[_, y]| (y - 2.0).abs() < 1e-9));
        assert!(
            result.exact,
            "a recognized curve differentiates symbolically"
        );
        assert_eq!(result.latex, "f(x) = 2", "{}", result.latex);
        assert!(result.approximation.is_none(), "exact — no approximation");
    }

    #[test]
    fn apply_calculus_differentiates_a_typed_cubic_to_a_clean_parabola() {
        // The reported bug: d/dx of typed x³ points was a lumpy numeric derivative.
        // Recognized as x³, it differentiates to exactly 3x² (a smooth parabola).
        let result = apply_calculus(
            integer_points(-3, 3, |x| x.powi(3)),
            vec![CalcOp::Differentiate],
        )
        .unwrap();
        assert!(result.exact);
        assert_eq!(result.latex, "f(x) = 3x^{2}", "{}", result.latex);
    }

    #[test]
    fn apply_calculus_integrates_a_typed_cubic_without_a_bogus_pretty_form() {
        // The old numeric path offered a wrong closed form (`… + cos x`) for ∫x³,
        // whose true value x⁴/4 is outside the basis. Symbolically it is exact and
        // carries no misleading approximation.
        let result = apply_calculus(
            integer_points(-3, 3, |x| x.powi(3)),
            vec![CalcOp::Integrate],
        )
        .unwrap();
        assert!(result.exact);
        assert!(result.latex.contains("0.25x^{4}"), "{}", result.latex);
        assert!(result.approximation.is_none(), "no bogus approximation");
    }

    #[test]
    fn apply_calculus_falls_back_to_numeric_for_an_unrecognized_squiggle() {
        // A curve no closed form describes still differentiates numerically
        // (piecewise), flagged not-exact so the UI keeps its "corners" note.
        let knots = vec![
            dto(0.0, 0.0, None),
            dto(1.0, 2.0, None),
            dto(2.0, -2.0, None),
            dto(3.0, 2.0, None),
            dto(4.0, -2.0, None),
            dto(5.0, 2.0, None),
            dto(6.0, 0.0, None),
        ];
        let result = apply_calculus(knots, vec![CalcOp::Differentiate]).unwrap();
        assert!(!result.exact, "an unrecognized curve uses the numeric path");
        assert!(result.latex.contains("\\begin{cases}"));
    }

    #[test]
    fn apply_calculus_chains_integrate_then_differentiate_back_to_the_curve() {
        // FTC end-to-end: d/dx ∫ f = f, so the chained polyline matches the drawn
        // curve's within tolerance.
        let knots = vec![
            dto(0.0, 0.0, None),
            dto(1.0, 2.0, None),
            dto(2.0, -1.0, None),
        ];
        let original = apply_calculus(knots.clone(), vec![]).unwrap();
        let recovered =
            apply_calculus(knots, vec![CalcOp::Integrate, CalcOp::Differentiate]).unwrap();
        for (o, r) in original.polyline.iter().zip(&recovered.polyline) {
            assert!((o[1] - r[1]).abs() < 1e-6, "{} vs {}", o[1], r[1]);
        }
    }

    #[test]
    fn save_then_open_round_trips_the_curve_through_a_file() {
        let knots = vec![
            dto(0.0, 0.0, None),
            dto(1.0, 2.0, Some(0.5)),
            dto(2.0, -1.0, None),
        ];
        let path = std::env::temp_dir()
            .join(format!("unplot_roundtrip_{}.unplot", std::process::id()))
            .to_string_lossy()
            .into_owned();
        save_curve(path.clone(), knots).unwrap();
        let opened = open_curve(path.clone()).unwrap();
        std::fs::remove_file(&path).ok();
        assert_eq!(opened.knots.len(), 3);
        assert_eq!(opened.knots[1].tangent, Some(0.5)); // user tangent survives
        assert!(opened.polyline.len() >= 2);
    }

    #[test]
    fn open_curve_reports_a_missing_file() {
        assert!(open_curve("/no/such/unplot/file.unplot".to_string()).is_err());
    }

    fn dto(x: f64, y: f64, tangent: Option<f64>) -> KnotDto {
        KnotDto {
            x,
            y,
            tangent,
            slope: 0.0,
        }
    }
}
