//! Strategy 1: sparse fixed-basis least squares. The fewest-term combination of
//! the dictionary `{1, x, x², x³, sin x, cos x, eˣ, ln x}` whose error clears the
//! gate — nails "basically x²" or "basically eˣ".

use super::{candidate, errors, solve, Candidate, MAX_TERMS, USABLE_CAP};

/// One dictionary entry: how to evaluate it and how it reads in LaTeX (the factor
/// that a coefficient multiplies; empty for the constant term).
struct Basis {
    eval: fn(f64) -> f64,
    latex: &'static str,
}

fn one(_: f64) -> f64 {
    1.0
}
fn linear(x: f64) -> f64 {
    x
}
fn square(x: f64) -> f64 {
    x * x
}
fn cube(x: f64) -> f64 {
    x * x * x
}

const DICTIONARY: &[Basis] = &[
    Basis {
        eval: one,
        latex: "",
    },
    Basis {
        eval: linear,
        latex: "x",
    },
    Basis {
        eval: square,
        latex: "x^{2}",
    },
    Basis {
        eval: cube,
        latex: "x^{3}",
    },
    Basis {
        eval: f64::sin,
        latex: "\\sin x",
    },
    Basis {
        eval: f64::cos,
        latex: "\\cos x",
    },
    Basis {
        eval: f64::exp,
        latex: "e^{x}",
    },
    Basis {
        eval: f64::ln,
        latex: "\\ln x",
    },
];

/// The fewest-term basis fit whose error clears the gate, or `None`.
pub(super) fn basis_candidate(
    fit_x: &[f64],
    fit_y: &[f64],
    err_x: &[f64],
    err_y: &[f64],
    tolerance: f64,
) -> Option<Candidate> {
    let usable = usable_basis(fit_x);
    // A clean fit is only *evidence* of a shape when it has more points than free
    // terms: an n-point set passes exactly through any n-term basis, so allowing
    // size == n would "recognize" noise and, on ties, pick an ugly exact fit (a
    // 2-point line read as `1 + 0.5x³`). Require at least one residual degree of
    // freedom — which only bites the knots path, as the spline path samples 120.
    let max_terms = MAX_TERMS
        .min(usable.len())
        .min(fit_x.len().saturating_sub(1));
    for size in 1..=max_terms {
        let best = combinations(&usable, size)
            .into_iter()
            .filter_map(|subset| fit_basis_subset(&subset, fit_x, fit_y, err_x, err_y))
            .filter(|form| form.max_error <= tolerance)
            .min_by(|a, b| a.rms_error.total_cmp(&b.rms_error));
        if best.is_some() {
            return best; // fewer terms wins, so the first non-empty size is best
        }
    }
    None
}

/// Fit `subset` of the dictionary to the samples, prettify the coefficients, and
/// measure error on the denser grid. `None` if the solve fails or every term
/// rounds away.
fn fit_basis_subset(
    subset: &[usize],
    fit_x: &[f64],
    fit_y: &[f64],
    err_x: &[f64],
    err_y: &[f64],
) -> Option<Candidate> {
    let coeffs = solve(fit_x, fit_y, subset.len(), |i, j| {
        (DICTIONARY[subset[j]].eval)(fit_x[i])
    })?;
    let approx = |x: f64| -> f64 {
        subset
            .iter()
            .zip(&coeffs)
            .map(|(&idx, &c)| c * (DICTIONARY[idx].eval)(x))
            .sum()
    };
    let (max_error, rms_error) = errors(&approx, err_x, err_y)?;
    let pairs: Vec<(f64, String)> = subset
        .iter()
        .zip(&coeffs)
        .map(|(&idx, &c)| (c, DICTIONARY[idx].latex.to_string()))
        .collect();
    candidate(&pairs, max_error, rms_error)
}

/// Dictionary indices whose values are finite and bounded over the domain, so a
/// domain-restricted function (`ln` on x ≤ 0, `exp` overflowing) is skipped.
fn usable_basis(fit_x: &[f64]) -> Vec<usize> {
    (0..DICTIONARY.len())
        .filter(|&j| {
            fit_x.iter().all(|&x| {
                let v = (DICTIONARY[j].eval)(x);
                v.is_finite() && v.abs() <= USABLE_CAP
            })
        })
        .collect()
}

/// Every `size`-element combination of `items`, in a deterministic order.
fn combinations(items: &[usize], size: usize) -> Vec<Vec<usize>> {
    let n = items.len();
    let mut out = Vec::new();
    if size == 0 || size > n {
        return out;
    }
    let mut c: Vec<usize> = (0..size).collect();
    loop {
        out.push(c.iter().map(|&i| items[i]).collect());
        let mut i = size;
        loop {
            if i == 0 {
                return out;
            }
            i -= 1;
            if c[i] < n - size + i {
                break;
            }
        }
        c[i] += 1;
        for j in i + 1..size {
            c[j] = c[j - 1] + 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combinations_are_complete_and_ordered() {
        assert_eq!(
            combinations(&[0, 1, 2], 2),
            vec![vec![0, 1], vec![0, 2], vec![1, 2]]
        );
        assert_eq!(combinations(&[5, 6], 1), vec![vec![5], vec![6]]);
        assert!(combinations(&[0], 2).is_empty());
    }
}
