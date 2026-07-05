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

#[cfg(test)]
mod tests {
    /// Phase 0 smoke test: proves the crate compiles, links, and runs under the
    /// test harness. Replaced by the real engine tests in Phase 1.
    #[test]
    fn crate_compiles_and_links() {
        assert_eq!(2 + 2, 4);
    }
}
