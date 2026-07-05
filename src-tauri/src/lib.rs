use serde::Serialize;

/// The result of fitting a drawn stroke: the resampled knots and a dense
/// polyline of the smooth spline, both in world coordinates.
#[derive(Serialize)]
struct FittedCurve {
    knots: Vec<[f64; 2]>,
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
    let points: Vec<(f64, f64)> = samples.iter().map(|&[x, y]| (x, y)).collect();
    let knots = curve_engine::resample(&points, tolerance);
    let curve = curve_engine::Curve::new(knots).map_err(|error| error.to_string())?;
    let spline = curve.fit();
    Ok(FittedCurve {
        knots: curve.knots().iter().map(|knot| [knot.x, knot.y]).collect(),
        polyline: spline
            .polyline(POLYLINE_POINTS)
            .iter()
            .map(|&(x, y)| [x, y])
            .collect(),
    })
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
        .invoke_handler(tauri::generate_handler![engine_version, fit_curve])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::fit_curve;

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
}
