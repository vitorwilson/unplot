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
            fit_curve,
            extend_curve,
            refit_curve
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::{extend_curve, fit_curve, refit_curve, KnotDto};

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

    fn dto(x: f64, y: f64, tangent: Option<f64>) -> KnotDto {
        KnotDto {
            x,
            y,
            tangent,
            slope: 0.0,
        }
    }
}
