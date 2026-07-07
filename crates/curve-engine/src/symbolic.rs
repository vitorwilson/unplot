//! A small symbolic layer: the closed forms the [`crate::approximate`] module
//! recognizes, as structured expressions that differentiate, integrate, evaluate,
//! and render exactly. Used so calculus on a *recognized* curve is clean and
//! correct — d/dx of x³ is exactly 3x², not the lumpy numeric derivative of the
//! smoothed spline — falling back to numeric spline calculus when nothing is
//! recognized (docs/PLAN.md, Phase 7 × Phase 5).
//!
//! Not a general CAS: `Expr` holds only what the approximator produces (a sum of
//! simple terms, or one rational `P/Q`) and their derivatives and integrals.
//! Differentiation is total; integration returns `None` for the forms it cannot
//! keep closed (a rational, or a repeated integral of `ln`), so the caller falls
//! back rather than inventing a wrong answer.

use crate::coeffs::{fmt_num, join_terms, DISPLAY_EPS};

/// Angular frequencies this close to 1 render as `sin x` rather than `sin(1x)`.
const UNIT_OMEGA_EPS: f64 = 1e-6;

/// One additive term of a closed form.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Term {
    /// `coeff · x^power` (power may be negative: `1/x` from `d(ln x)`).
    Power { coeff: f64, power: i32 },
    /// `coeff · sin(omega·x)`.
    Sin { coeff: f64, omega: f64 },
    /// `coeff · cos(omega·x)`.
    Cos { coeff: f64, omega: f64 },
    /// `coeff · eˣ`.
    Exp { coeff: f64 },
    /// `coeff · ln x`.
    Ln { coeff: f64 },
    /// `coeff · x·ln x` (only produced by `∫ ln x`).
    XLn { coeff: f64 },
}

/// A recognized closed form: a sum of simple terms, or a rational `P(x)/Q(x)`
/// (coefficient vectors are low-order first, so `[2, 0, 1]` is `2 + x²`).
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Sum(Vec<Term>),
    Rational { num: Vec<f64>, den: Vec<f64> },
}

impl Expr {
    /// Evaluate `f(x)`. May be non-finite (`ln x` at x ≤ 0, a pole of a rational);
    /// callers gate on that.
    pub fn eval(&self, x: f64) -> f64 {
        match self {
            Expr::Sum(terms) => terms.iter().map(|t| t.eval(x)).sum(),
            Expr::Rational { num, den } => horner(num, x) / horner(den, x),
        }
    }

    /// The exact derivative — total; every recognized form has one.
    pub fn differentiate(&self) -> Expr {
        match self {
            Expr::Sum(terms) => simplified(terms.iter().flat_map(diff_term).collect()),
            Expr::Rational { num, den } => diff_rational(num, den),
        }
    }

    /// The exact antiderivative `F` with `F(x_start) = 0`, or `None` when the form
    /// has no closed antiderivative here (a rational, `∫∫ ln`, or an anchor that
    /// is non-finite — e.g. `ln x` integrated from x ≤ 0).
    pub fn integrate(&self, x_start: f64) -> Option<Expr> {
        let Expr::Sum(terms) = self else {
            return None; // a rational integral needs partial fractions — fall back
        };
        let mut antiderivative = Vec::new();
        for term in terms {
            antiderivative.extend(integ_term(term)?);
        }
        let anchored = anchor_at_zero(simplified(antiderivative), x_start)?;
        Some(anchored)
    }

    /// KaTeX-ready form, e.g. `3x^{2}` or `\frac{1}{x}` (no `f(x) =` prefix).
    pub fn to_latex(&self) -> String {
        match self {
            Expr::Sum(terms) => join_terms(terms.iter().map(|t| (t.coeff(), t.latex_factor()))),
            Expr::Rational { num, den } => {
                format!(
                    "\\frac{{{}}}{{{}}}",
                    polynomial_latex(num),
                    polynomial_latex(den)
                )
            }
        }
    }

    /// Wolfram-Language form, e.g. `3x^2` or `(1)/(x)`.
    pub fn to_wolfram(&self) -> String {
        match self {
            Expr::Sum(terms) => join_terms(terms.iter().map(|t| (t.coeff(), t.wolfram_factor()))),
            Expr::Rational { num, den } => {
                format!(
                    "({})/({})",
                    polynomial_wolfram(num),
                    polynomial_wolfram(den)
                )
            }
        }
    }

    /// How many terms survive rounding — a proxy for how busy the form reads,
    /// used when the approximator's strategies compete.
    pub fn term_count(&self) -> usize {
        match self {
            Expr::Sum(terms) => terms
                .iter()
                .filter(|t| t.coeff().abs() >= DISPLAY_EPS)
                .count(),
            Expr::Rational { num, den } => nonzero(num) + nonzero(den),
        }
    }
}

impl Term {
    fn coeff(&self) -> f64 {
        match *self {
            Term::Power { coeff, .. }
            | Term::Sin { coeff, .. }
            | Term::Cos { coeff, .. }
            | Term::Exp { coeff }
            | Term::Ln { coeff }
            | Term::XLn { coeff } => coeff,
        }
    }

    fn eval(&self, x: f64) -> f64 {
        match *self {
            Term::Power { coeff, power } => coeff * x.powi(power),
            Term::Sin { coeff, omega } => coeff * (omega * x).sin(),
            Term::Cos { coeff, omega } => coeff * (omega * x).cos(),
            Term::Exp { coeff } => coeff * x.exp(),
            Term::Ln { coeff } => coeff * x.ln(),
            Term::XLn { coeff } => coeff * x * x.ln(),
        }
    }

    /// The LaTeX factor a coefficient multiplies (empty for the constant).
    fn latex_factor(&self) -> String {
        match *self {
            Term::Power { power, .. } => monomial_latex(power),
            Term::Sin { omega, .. } => trig_latex("\\sin", omega),
            Term::Cos { omega, .. } => trig_latex("\\cos", omega),
            Term::Exp { .. } => "e^{x}".to_string(),
            Term::Ln { .. } => "\\ln x".to_string(),
            Term::XLn { .. } => "x\\ln x".to_string(),
        }
    }

    /// The Wolfram-Language factor a coefficient multiplies.
    fn wolfram_factor(&self) -> String {
        match *self {
            Term::Power { power, .. } => monomial_wolfram(power),
            Term::Sin { omega, .. } => trig_wolfram("Sin", omega),
            Term::Cos { omega, .. } => trig_wolfram("Cos", omega),
            Term::Exp { .. } => "E^x".to_string(),
            Term::Ln { .. } => "Log[x]".to_string(),
            Term::XLn { .. } => "xLog[x]".to_string(),
        }
    }
}

// --- Differentiation ------------------------------------------------------------

/// The derivative of one term, as zero, one, or two terms (the product rule on
/// `x·ln x` yields two).
fn diff_term(term: &Term) -> Vec<Term> {
    match *term {
        Term::Power { power: 0, .. } => vec![],
        Term::Power { coeff, power } => vec![Term::Power {
            coeff: coeff * power as f64,
            power: power - 1,
        }],
        Term::Sin { coeff, omega } => vec![Term::Cos {
            coeff: coeff * omega,
            omega,
        }],
        Term::Cos { coeff, omega } => vec![Term::Sin {
            coeff: -coeff * omega,
            omega,
        }],
        Term::Exp { coeff } => vec![Term::Exp { coeff }],
        Term::Ln { coeff } => vec![Term::Power { coeff, power: -1 }],
        Term::XLn { coeff } => vec![Term::Ln { coeff }, Term::Power { coeff, power: 0 }],
    }
}

/// The quotient rule `(P/Q)' = (P'Q − PQ')/Q²`, kept as a rational (unreduced).
fn diff_rational(num: &[f64], den: &[f64]) -> Expr {
    let dnum = poly_derivative(num);
    let dden = poly_derivative(den);
    let numerator = poly_sub(&poly_mul(&dnum, den), &poly_mul(num, &dden));
    Expr::Rational {
        num: numerator,
        den: poly_mul(den, den),
    }
}

// --- Integration ----------------------------------------------------------------

/// The antiderivative of one term (constant 0), or `None` if it has no closed
/// form here (`∫ x·ln x` needs `x²·ln x`, which the term set does not carry).
fn integ_term(term: &Term) -> Option<Vec<Term>> {
    Some(match *term {
        Term::Power { coeff, power: -1 } => vec![Term::Ln { coeff }],
        Term::Power { coeff, power } => vec![Term::Power {
            coeff: coeff / (power as f64 + 1.0),
            power: power + 1,
        }],
        Term::Sin { coeff, omega } => vec![Term::Cos {
            coeff: -coeff / omega,
            omega,
        }],
        Term::Cos { coeff, omega } => vec![Term::Sin {
            coeff: coeff / omega,
            omega,
        }],
        Term::Exp { coeff } => vec![Term::Exp { coeff }],
        Term::Ln { coeff } => vec![
            Term::XLn { coeff },
            Term::Power {
                coeff: -coeff,
                power: 1,
            },
        ],
        Term::XLn { .. } => return None,
    })
}

/// Add the constant that makes `F(x_start) = 0`; `None` if `F(x_start)` is not
/// finite (so we never anchor a `ln` integral against x ≤ 0).
fn anchor_at_zero(antiderivative: Expr, x_start: f64) -> Option<Expr> {
    let offset = antiderivative.eval(x_start);
    if !offset.is_finite() {
        return None;
    }
    let Expr::Sum(mut terms) = antiderivative else {
        return None;
    };
    terms.insert(
        0,
        Term::Power {
            coeff: -offset,
            power: 0,
        },
    );
    Some(simplified(terms))
}

// --- Simplification -------------------------------------------------------------

/// Merge like terms (same kind, power, and frequency) and drop the ones that
/// round to zero, so a cancellation like `x − x` disappears from the output.
fn simplified(terms: Vec<Term>) -> Expr {
    let mut merged: Vec<Term> = Vec::new();
    for term in terms {
        match merged.iter_mut().find(|m| same_shape(m, &term)) {
            Some(existing) => *existing = add_coeff(*existing, term.coeff()),
            None => merged.push(term),
        }
    }
    merged.retain(|t| t.coeff().abs() >= DISPLAY_EPS);
    Expr::Sum(merged)
}

/// Whether two terms differ only in coefficient, so they can be summed.
fn same_shape(a: &Term, b: &Term) -> bool {
    match (a, b) {
        (Term::Power { power: p, .. }, Term::Power { power: q, .. }) => p == q,
        (Term::Sin { omega: p, .. }, Term::Sin { omega: q, .. })
        | (Term::Cos { omega: p, .. }, Term::Cos { omega: q, .. }) => {
            (p - q).abs() < UNIT_OMEGA_EPS
        }
        (Term::Exp { .. }, Term::Exp { .. })
        | (Term::Ln { .. }, Term::Ln { .. })
        | (Term::XLn { .. }, Term::XLn { .. }) => true,
        _ => false,
    }
}

/// The term `existing` with `delta` added to its coefficient.
fn add_coeff(existing: Term, delta: f64) -> Term {
    let sum = existing.coeff() + delta;
    match existing {
        Term::Power { power, .. } => Term::Power { coeff: sum, power },
        Term::Sin { omega, .. } => Term::Sin { coeff: sum, omega },
        Term::Cos { omega, .. } => Term::Cos { coeff: sum, omega },
        Term::Exp { .. } => Term::Exp { coeff: sum },
        Term::Ln { .. } => Term::Ln { coeff: sum },
        Term::XLn { .. } => Term::XLn { coeff: sum },
    }
}

// --- Rendering helpers ----------------------------------------------------------

/// A bare monomial in LaTeX: empty constant, `x`, `x^{k}` (braces for KaTeX);
/// a negative power (`1/x` from `d(ln x)`) reads as `x^{-1}`.
fn monomial_latex(power: i32) -> String {
    match power {
        0 => String::new(),
        1 => "x".to_string(),
        k => format!("x^{{{k}}}"),
    }
}

/// A bare monomial in Wolfram: empty constant, `x`, `x^k` (no braces).
fn monomial_wolfram(power: i32) -> String {
    match power {
        0 => String::new(),
        1 => "x".to_string(),
        k => format!("x^{k}"),
    }
}

/// A polynomial `Σ cₖ xᵏ` in LaTeX (`1 + x^{2}`).
fn polynomial_latex(coeffs: &[f64]) -> String {
    join_terms(
        coeffs
            .iter()
            .enumerate()
            .map(|(k, &c)| (c, monomial_latex(k as i32))),
    )
}

/// A polynomial `Σ cₖ xᵏ` in Wolfram syntax (`1 + x^2`).
fn polynomial_wolfram(coeffs: &[f64]) -> String {
    join_terms(
        coeffs
            .iter()
            .enumerate()
            .map(|(k, &c)| (c, monomial_wolfram(k as i32))),
    )
}

/// A trig factor: `\sin x` at ω = 1, else `\sin(2x)`.
fn trig_latex(name: &str, omega: f64) -> String {
    if (omega - 1.0).abs() < UNIT_OMEGA_EPS {
        format!("{name} x")
    } else {
        format!("{name}({}x)", fmt_num(omega))
    }
}

/// A trig factor in Wolfram: `Sin[x]` at ω = 1, else `Sin[2x]`.
fn trig_wolfram(name: &str, omega: f64) -> String {
    if (omega - 1.0).abs() < UNIT_OMEGA_EPS {
        format!("{name}[x]")
    } else {
        format!("{name}[{}x]", fmt_num(omega))
    }
}

// --- Polynomial arithmetic (for the quotient rule) ------------------------------

fn horner(coeffs: &[f64], x: f64) -> f64 {
    coeffs.iter().rev().fold(0.0, |acc, &c| acc * x + c)
}

fn poly_derivative(a: &[f64]) -> Vec<f64> {
    a.iter()
        .enumerate()
        .skip(1)
        .map(|(k, &c)| k as f64 * c)
        .collect()
}

fn poly_mul(a: &[f64], b: &[f64]) -> Vec<f64> {
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

fn poly_sub(a: &[f64], b: &[f64]) -> Vec<f64> {
    let n = a.len().max(b.len());
    (0..n)
        .map(|i| a.get(i).copied().unwrap_or(0.0) - b.get(i).copied().unwrap_or(0.0))
        .collect()
}

fn nonzero(coeffs: &[f64]) -> usize {
    coeffs.iter().filter(|c| c.abs() >= DISPLAY_EPS).count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    fn poly(coeffs: &[f64]) -> Expr {
        Expr::Sum(
            coeffs
                .iter()
                .enumerate()
                .map(|(k, &c)| Term::Power {
                    coeff: c,
                    power: k as i32,
                })
                .collect(),
        )
    }

    fn close(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-9, "{a} ≈ {b}");
    }

    #[test]
    fn differentiates_a_cubic_to_a_parabola() {
        // d/dx x³ = 3x².
        let d = poly(&[0.0, 0.0, 0.0, 1.0]).differentiate();
        assert_eq!(d.to_latex(), "3x^{2}");
        close(d.eval(2.0), 12.0);
    }

    #[test]
    fn integrates_a_cubic_anchored_at_the_domain_start() {
        // ∫ x³ dx with F(-3) = 0  ⇒  x⁴/4 − 81/4.
        let f = poly(&[0.0, 0.0, 0.0, 1.0]).integrate(-3.0).unwrap();
        assert_eq!(f.to_latex(), "-20.25 + 0.25x^{4}");
        close(f.eval(-3.0), 0.0);
        close(f.eval(3.0), 0.0);
        close(f.eval(0.0), -20.25);
    }

    #[test]
    fn differentiation_undoes_integration() {
        let original = poly(&[1.0, -2.0, 3.0]); // 1 - 2x + 3x²
        let recovered = original.integrate(0.0).unwrap().differentiate();
        for i in 0..=10 {
            let x = -2.0 + 0.4 * i as f64;
            close(recovered.eval(x), original.eval(x));
        }
    }

    #[test]
    fn differentiates_and_integrates_a_wave() {
        let wave = Expr::Sum(vec![Term::Sin {
            coeff: 1.0,
            omega: 2.0,
        }]);
        // d/dx sin(2x) = 2cos(2x); ∫ sin(2x) = -0.5cos(2x) (+ anchor).
        assert_eq!(wave.differentiate().to_latex(), "2\\cos(2x)");
        assert!(wave
            .integrate(0.0)
            .unwrap()
            .to_latex()
            .contains("\\cos(2x)"));
        close(wave.differentiate().eval(0.0), 2.0);
    }

    #[test]
    fn differentiates_a_log_into_a_reciprocal() {
        // d/dx ln x = 1/x, rendered with a negative exponent.
        let d = Expr::Sum(vec![Term::Ln { coeff: 1.0 }]).differentiate();
        assert_eq!(d.to_latex(), "x^{-1}");
        close(d.eval(2.0), 0.5);
    }

    #[test]
    fn integrates_a_log() {
        // ∫ ln x dx = x ln x - x (anchored at x = 1, where it is 0 - 1 = -1).
        let f = Expr::Sum(vec![Term::Ln { coeff: 1.0 }])
            .integrate(1.0)
            .unwrap();
        close(f.eval(1.0), 0.0);
        close(f.eval(PI), PI * PI.ln() - PI + 1.0);
        assert!(f.to_latex().contains("x\\ln x"), "{}", f.to_latex());
    }

    #[test]
    fn differentiates_a_reciprocal_by_the_quotient_rule() {
        // d/dx (1/x) = -1/x².
        let d = Expr::Rational {
            num: vec![1.0],
            den: vec![0.0, 1.0],
        }
        .differentiate();
        close(d.eval(2.0), -0.25);
        assert_eq!(d.to_latex(), "\\frac{-1}{x^{2}}");
    }

    #[test]
    fn a_rational_integral_falls_back() {
        let rational = Expr::Rational {
            num: vec![1.0],
            den: vec![0.0, 1.0],
        };
        assert!(rational.integrate(1.0).is_none());
    }

    #[test]
    fn renders_wolfram_syntax() {
        assert_eq!(poly(&[0.0, 0.0, 3.0]).to_wolfram(), "3x^2");
        let wave = Expr::Sum(vec![Term::Cos {
            coeff: 2.0,
            omega: 1.0,
        }]);
        assert_eq!(wave.to_wolfram(), "2Cos[x]");
    }
}
