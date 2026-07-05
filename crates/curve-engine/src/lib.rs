//! Headless curve engine for "back-desmos".
//!
//! The source of truth for a drawing is a shape-preserving piecewise cubic
//! Hermite spline (PCHIP / Fritsch–Carlson tangents): C¹ everywhere, locally
//! editable, no overshoot. This crate is UI-free by design — fitting,
//! evaluation, calculus, and serialization live here and are unit-tested
//! without ever touching Tauri. See `docs/PLAN.md` for the roadmap.
//!
//! Phase 0 is scaffolding only. The `Knot` data model and spline fitting land
//! in Phase 1; nothing here yet does real math.

#![forbid(unsafe_code)]

mod curve;
mod error;
mod knot;
mod spline;

pub use curve::Curve;
pub use error::CurveError;
pub use knot::Knot;
pub use spline::{Segment, Spline};

/// Version of the curve engine, surfaced to the UI shell so it can show which
/// core build it is running against.
///
/// # Example
/// ```
/// assert!(!curve_engine::engine_version().is_empty());
/// ```
pub fn engine_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::engine_version;

    #[test]
    fn engine_version_matches_crate_version() {
        assert_eq!(engine_version(), env!("CARGO_PKG_VERSION"));
    }
}
