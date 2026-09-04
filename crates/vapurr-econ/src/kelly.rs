//! W* = max E[log bᵀX]
//!
//! Maximize expected log wealth, not average return. Arithmetic dumps into
//! the top name. Log spreads and rebalances — compounding is the geometric
//! mean, and volatility is fuel.

/// Project onto { b | b ≥ 0, 1ᵀb = 1 }. Duchi et al. 2008.
pub fn project_simplex(v: &mut [f64]) {
    let n = v.len();
    if n == 0 {
        return;
    }
    for x in v.iter_mut() {
        if !x.is_finite() {
            *x = 0.0;
        }
    }
    let mut u = v.to_vec();
    u.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    let mut css = 0.0;
    let mut rho = 0usize;
    for (j, uj) in u.iter().enumerate() {
        css += *uj;
        let t = (css - 1.0) / (j as f64 + 1.0);
        if *uj - t > 0.0 {
            rho = j;
        }
    }
    let s: f64 = u.iter().take(rho + 1).sum();
    let theta = (s - 1.0) / (rho as f64 + 1.0);
    for x in v.iter_mut() {
        *x = (*x - theta).max(0.0);
    }
    let z: f64 = v.iter().sum();
    if z > 1e-12 {
        for x in v.iter_mut() {
            *x /= z;
        }
    } else {
        let eq = 1.0 / n as f64;
        for x in v.iter_mut() {
            *x = eq;
        }
    }
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// Empirical log-optimal: max (1/T) Σ log(b · x_t) over the simplex.
/// One observation is arithmetic in disguise — fall back to 1/n.
pub fn log_optimal(xs: &[Vec<f64>], steps: usize) -> Vec<f64> {
    let n = xs.first().map(|x| x.len()).unwrap_or(0);
    if n == 0 {
        return Vec::new();
    }
    let eq = vec![1.0 / n as f64; n];
    if xs.len() < 2 || xs.iter().any(|x| x.len() != n) {
        return eq;
    }
    let mut b = eq.clone();
    let t = xs.len() as f64;
    let eta = 0.35;
    for _ in 0..steps.max(1) {
        let mut g = vec![0.0; n];
        for x in xs {
            let s = dot(&b, x).max(1e-12);
            for i in 0..n {
                g[i] += x[i] / s;
            }
        }
        for i in 0..n {
            b[i] += eta * (g[i] / t);
        }
        project_simplex(&mut b);
    }
    b
}

pub fn mean_log(b: &[f64], xs: &[Vec<f64>]) -> f64 {
    if xs.is_empty() {
        return 0.0;
    }
    let mut s = 0.0;
    for x in xs {
        s += dot(b, x).max(1e-12).ln();
    }
    s / xs.len() as f64
}

pub fn mean_arith(b: &[f64], xs: &[Vec<f64>]) -> f64 {
    if xs.is_empty() {
        return 0.0;
    }
    let mut s = 0.0;
    for x in xs {
        s += dot(b, x);
    }
    s / xs.len() as f64
}

/// Weight that arithmetic mean would pick: 100% in the best average name.
pub fn arith_dump(xs: &[Vec<f64>]) -> Vec<f64> {
    let n = xs.first().map(|x| x.len()).unwrap_or(0);
    if n == 0 {
        return Vec::new();
    }
    let mut mu = vec![0.0; n];
    for x in xs {
        if x.len() != n {
            return vec![1.0 / n as f64; n];
        }
        for i in 0..n {
            mu[i] += x[i];
        }
    }
    let t = xs.len().max(1) as f64;
    for m in mu.iter_mut() {
        *m /= t;
    }
    let mut best = 0usize;
    for i in 1..n {
        if mu[i] > mu[best] {
            best = i;
        }
    }
    let mut b = vec![0.0; n];
    b[best] = 1.0;
    b
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-6, "{a} != {b}");
    }

    #[test]
    fn simplex_sums_to_one() {
        let mut v = vec![3.0, -1.0, 0.5];
        project_simplex(&mut v);
        close(v.iter().sum::<f64>(), 1.0);
        assert!(v.iter().all(|x| *x >= -1e-12));
    }

    #[test]
    fn one_period_is_uniform() {
        let b = log_optimal(&[vec![1.0, 1.2, 0.8]], 80);
        assert_eq!(b.len(), 3);
        for w in &b {
            close(*w, 1.0 / 3.0);
        }
    }

    #[test]
    fn log_does_not_dump_into_the_top_stock() {
        // Dollar always 1. Hot name: 1.8 or 0.55. Arithmetic loves it.
        // Log will not go 100% — a wipe day compounds.
        let mut xs = Vec::new();
        for i in 0..20 {
            if i % 2 == 0 {
                xs.push(vec![1.0, 1.8]);
            } else {
                xs.push(vec![1.0, 0.55]);
            }
        }
        let dump = arith_dump(&xs);
        assert!(dump[1] > 0.99, "arithmetic should pick the hot name");
        let b = log_optimal(&xs, 600);
        assert!(b[1] < 0.92, "log must keep a dollar sleeve, got {b:?}");
        assert!(b[0] > 0.08, "dollar sleeve, got {b:?}");
        close(b.iter().sum::<f64>(), 1.0);
        let g_log = mean_log(&b, &xs);
        let g_dump = mean_log(&dump, &xs);
        assert!(
            g_log + 1e-9 >= g_dump,
            "log book {g_log} should beat dump {g_dump}"
        );
    }
}
