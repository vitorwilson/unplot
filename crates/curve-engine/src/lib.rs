//! Headless curve engine for "unplot".
//!
//! The source of truth for a drawing is a shape-preserving piecewise cubic
//! Hermite spline (PCHIP / Fritsch–Carlson tangents): C¹ everywhere, locally
//! editable, no overshoot. This crate is UI-free by design — everything here is
//! unit-tested without ever touching Tauri. See `docs/PLAN.md` for the roadmap.
//!
//! What it does, module by module:
//! - [`Curve`]/[`Knot`] — the validated function model and its invariants.
//! - [`Spline`] — the fitted spline: evaluation, sampling, boundary slopes.
//! - [`calculus`] — exact per-segment differentiation and integration.
//! - [`approximate`] — the "prettier function": a closed form for a curve that
//!   really is one (polynomial, wave, or rational), always error-gated.
//! - [`symbolic`] — the recognized form as an expression, so calculus on it is
//!   exact (d/dx of a drawn x³ is 3x², not a numeric approximation).
//! - [`latex`]/[`export`] — render a spline as LaTeX, Desmos, or Wolfram.
//! - [`document`] — the versioned `.unplot` save format.
//! - [`resample`]/[`validate`] — thin a raw stroke and enforce the "is a
//!   function" hard-block.

#![forbid(unsafe_code)]

pub mod approximate;
pub mod calculus;
mod coeffs;
mod curve;
pub mod document;
mod error;
pub mod export;
mod knot;
pub mod latex;
mod poly;
mod resample;
mod spline;
pub mod symbolic;
mod validate;

pub use curve::Curve;
pub use error::CurveError;
pub use knot::Knot;
pub use resample::resample;
pub use spline::{Segment, Spline};
pub use validate::{advances_in_x, edit_keeps_order, within_slope_cap};

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
