//! Versioned, cross-platform document format for a drawn curve. The file stores
//! the *source of truth* — the ordered knots (x, y, optional user tangent) and
//! the domain — as JSON, never a rendered image, so a saved curve reopens fully
//! editable and re-derives its LaTeX and calculus deterministically
//! (docs/PLAN.md, Phase 6).

use crate::{Curve, CurveError, Knot};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Current on-disk schema version. Bump it when the stored shape changes;
/// [`from_json`] refuses versions newer than this build understands and is the
/// single place to migrate older versions forward.
pub const SCHEMA_VERSION: u32 = 1;

/// The file extension for a saved curve (JSON inside), distinct so the OS and the
/// open dialog can recognize an unplot document.
pub const FILE_EXTENSION: &str = "unplot";

/// One knot as stored: position and an optional user-set tangent (`None` = the
/// fitter chooses the slope). The *effective* rendered slope is deliberately not
/// stored — it is re-derived on load, keeping the file the minimal source of
/// truth.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct KnotRecord {
    pub x: f64,
    pub y: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tangent: Option<f64>,
}

/// Provenance metadata. Extensible: new optional fields with `#[serde(default)]`
/// keep older files loadable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Metadata {
    /// The unplot build that wrote the file, for diagnosing cross-version issues.
    pub app_version: String,
}

/// A saved unplot document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub schema_version: u32,
    pub metadata: Metadata,
    pub knots: Vec<KnotRecord>,
    /// The closed interval the curve spans, `[x_first, x_last]`. Stored for
    /// readability and validation; the knots remain canonical.
    pub domain: [f64; 2],
}

/// Why a document failed to load.
#[derive(Debug)]
pub enum DocError {
    /// The bytes are not valid JSON in the document shape.
    Parse(String),
    /// The file's schema is newer than this build can read.
    UnsupportedVersion(u32),
    /// The stored knots do not form a valid function (e.g. x not increasing).
    InvalidCurve(CurveError),
}

impl fmt::Display for DocError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DocError::Parse(msg) => write!(f, "not a valid unplot document: {msg}"),
            DocError::UnsupportedVersion(v) => write!(
                f,
                "unplot document schema v{v} is newer than this app supports (v{SCHEMA_VERSION}); update unplot to open it"
            ),
            DocError::InvalidCurve(err) => write!(f, "document does not describe a valid curve: {err}"),
        }
    }
}

impl std::error::Error for DocError {}

impl Document {
    /// Capture a curve as a document at the current schema version.
    pub fn from_curve(curve: &Curve) -> Document {
        let knots: Vec<KnotRecord> = curve
            .knots()
            .iter()
            .map(|k| KnotRecord {
                x: k.x,
                y: k.y,
                tangent: k.tangent,
            })
            .collect();
        let ends = curve.knots();
        Document {
            schema_version: SCHEMA_VERSION,
            metadata: Metadata {
                app_version: env!("CARGO_PKG_VERSION").to_string(),
            },
            domain: [ends[0].x, ends[ends.len() - 1].x],
            knots,
        }
    }

    /// Rebuild the editable curve, re-validating that the knots form a function.
    pub fn into_curve(self) -> Result<Curve, DocError> {
        let knots = self
            .knots
            .iter()
            .map(|r| match r.tangent {
                Some(m) => Knot::with_tangent(r.x, r.y, m),
                None => Knot::new(r.x, r.y),
            })
            .collect();
        Curve::new(knots).map_err(DocError::InvalidCurve)
    }

    /// Serialize to pretty JSON for writing to disk.
    pub fn to_json(&self) -> String {
        // Serialization of plain owned data cannot fail.
        serde_json::to_string_pretty(self).expect("Document serializes to JSON")
    }
}

/// Only the version, read first so a bad/newer file fails clearly before we try
/// to parse a shape that may have changed.
#[derive(Deserialize)]
struct VersionProbe {
    schema_version: u32,
}

/// Parse a document, checking the schema version before the full shape. Newer
/// versions are refused; this is where future migrations from older versions go.
pub fn from_json(json: &str) -> Result<Document, DocError> {
    let probe: VersionProbe =
        serde_json::from_str(json).map_err(|e| DocError::Parse(e.to_string()))?;
    if probe.schema_version > SCHEMA_VERSION {
        return Err(DocError::UnsupportedVersion(probe.schema_version));
    }
    // Only v1 exists today. When v2 lands, branch on `probe.schema_version` and
    // migrate the older shape forward here before returning a current Document.
    serde_json::from_str(json).map_err(|e| DocError::Parse(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Curve {
        Curve::new(vec![
            Knot::new(0.0, 0.0),
            Knot::with_tangent(1.0, 2.0, 0.5),
            Knot::new(2.5, -1.0),
        ])
        .unwrap()
    }

    #[test]
    fn round_trips_a_curve_through_json() {
        let curve = sample();
        let json = Document::from_curve(&curve).to_json();
        let reopened = from_json(&json).unwrap().into_curve().unwrap();
        assert_eq!(reopened.knots(), curve.knots());
    }

    #[test]
    fn preserves_user_tangents_and_their_absence() {
        let reopened = from_json(&Document::from_curve(&sample()).to_json())
            .unwrap()
            .into_curve()
            .unwrap();
        assert_eq!(reopened.knots()[0].tangent, None);
        assert_eq!(reopened.knots()[1].tangent, Some(0.5));
    }

    #[test]
    fn domain_matches_the_knot_span() {
        assert_eq!(Document::from_curve(&sample()).domain, [0.0, 2.5]);
    }

    #[test]
    fn json_names_the_schema_and_omits_absent_tangents() {
        let json = Document::from_curve(&sample()).to_json();
        assert!(json.contains("\"schema_version\": 1"), "json: {json}");
        // The tangent field appears once — only for the knot that has one.
        assert_eq!(json.matches("\"tangent\"").count(), 1, "json: {json}");
    }

    #[test]
    fn refuses_a_newer_schema_version() {
        let json = r#"{"schema_version": 999, "metadata": {"app_version": "9"},
            "knots": [{"x": 0.0, "y": 0.0}], "domain": [0.0, 0.0]}"#;
        assert!(matches!(
            from_json(json),
            Err(DocError::UnsupportedVersion(999))
        ));
    }

    #[test]
    fn rejects_malformed_json() {
        assert!(matches!(from_json("not json {"), Err(DocError::Parse(_))));
    }

    #[test]
    fn rejects_knots_that_are_not_a_function() {
        // x goes backward — into_curve must refuse it as an invalid function.
        let json = r#"{"schema_version": 1, "metadata": {"app_version": "1"},
            "knots": [{"x": 1.0, "y": 0.0}, {"x": 0.0, "y": 1.0}], "domain": [1.0, 0.0]}"#;
        let doc = from_json(json).unwrap();
        assert!(matches!(doc.into_curve(), Err(DocError::InvalidCurve(_))));
    }
}
