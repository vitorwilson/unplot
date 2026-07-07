use crate::dto::{render, to_knots, FittedCurve, KnotDto};
use curve_engine::Curve;

/// Save the drawn curve to `path` as a versioned `.unplot` document. The
/// frontend picks `path` via the native save dialog; only the knots (the source
/// of truth) are written, so the file reopens fully editable.
#[tauri::command]
pub(crate) fn save_curve(path: String, knots: Vec<KnotDto>) -> Result<(), String> {
    let curve = Curve::new(to_knots(&knots)).map_err(|error| error.to_string())?;
    let json = curve_engine::document::Document::from_curve(&curve).to_json();
    std::fs::write(&path, json).map_err(|error| format!("could not write {path}: {error}"))
}

/// Open a `.unplot` document from `path` and return the fitted curve for editing.
/// Errors (as a message) if the file is missing, malformed, from a newer schema,
/// or does not describe a valid function.
#[tauri::command]
pub(crate) fn open_curve(path: String) -> Result<FittedCurve, String> {
    let json = std::fs::read_to_string(&path)
        .map_err(|error| format!("could not read {path}: {error}"))?;
    let curve = curve_engine::document::from_json(&json)
        .map_err(|error| error.to_string())?
        .into_curve()
        .map_err(|error| error.to_string())?;
    Ok(render(&curve))
}

#[cfg(test)]
mod tests {
    use super::{open_curve, save_curve};
    use crate::dto::dto;

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
}
