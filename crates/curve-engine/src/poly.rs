//! Tiny polynomial arithmetic over coefficient vectors, low-order first, so
//! `[2.0, 0.0, 1.0]` is `2 + x²`. Shared by the symbolic layer (the quotient
//! rule) and the rational approximator (evaluating `P/Q`) so the rules live in
//! one place, not copied per module.

/// Evaluate `Σ cₖ xᵏ` by Horner's method.
pub(crate) fn horner(coeffs: &[f64], x: f64) -> f64 {
    coeffs.iter().rev().fold(0.0, |acc, &c| acc * x + c)
}

/// The derivative `Σ_{k≥1} k·cₖ xᵏ⁻¹`. A constant (or empty) polynomial gives the
/// empty vector, which the other operations treat as zero.
pub(crate) fn derivative(a: &[f64]) -> Vec<f64> {
    a.iter()
        .enumerate()
        .skip(1)
        .map(|(k, &c)| k as f64 * c)
        .collect()
}

/// The product `a·b` (convolution of the coefficients).
pub(crate) fn mul(a: &[f64], b: &[f64]) -> Vec<f64> {
    if a.is_empty() || b.is_empty() {
        return vec![];
    }
    let mut out = vec![0.0; a.len() + b.len() - 1];
    for (i, &ai) in a.iter().enumerate() {
        for (j, &bj) in b.iter().enumerate() {
            out[i + j] += ai * bj;
        }
    }
    out
}

/// The difference `a − b`, padding the shorter polynomial with zeros.
pub(crate) fn sub(a: &[f64], b: &[f64]) -> Vec<f64> {
    let n = a.len().max(b.len());
    (0..n)
        .map(|i| a.get(i).copied().unwrap_or(0.0) - b.get(i).copied().unwrap_or(0.0))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horner_evaluates_low_order_first() {
        // 2 + 3x + x²  at x = 2  = 2 + 6 + 4 = 12.
        assert_eq!(horner(&[2.0, 3.0, 1.0], 2.0), 12.0);
        assert_eq!(horner(&[], 5.0), 0.0);
    }

    #[test]
    fn derivative_lowers_the_degree() {
        // d/dx (2 + 3x + x²) = 3 + 2x.
        assert_eq!(derivative(&[2.0, 3.0, 1.0]), vec![3.0, 2.0]);
        assert!(derivative(&[7.0]).is_empty()); // a constant → zero
    }

    #[test]
    fn mul_convolves_and_sub_pads() {
        // (1 + x)(1 - x) = 1 - x².
        assert_eq!(mul(&[1.0, 1.0], &[1.0, -1.0]), vec![1.0, 0.0, -1.0]);
        assert!(mul(&[1.0], &[]).is_empty());
        // (1 + x + x²) - (x) = 1 + x².
        assert_eq!(sub(&[1.0, 1.0, 1.0], &[0.0, 1.0]), vec![1.0, 0.0, 1.0]);
    }
}
