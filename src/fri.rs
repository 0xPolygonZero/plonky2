/// Somewhat arbitrary. Smaller values will increase delta, but with diminishing returns,
/// while increasing L, potentially requiring more challenge points.
const EPSILON: f64 = 0.01;

fn fri_delta(rate_log: usize, conjecture: bool) -> f64 {
    let rate = (1 << rate_log) as f64;
    if conjecture {
        todo!()
    } else {
        return 1.0 - rate.sqrt() - EPSILON;
    }
}

fn fri_l(rate_log: usize, conjecture: bool) -> f64 {
    let rate = (1 << rate_log) as f64;
    if conjecture {
        todo!()
    } else {
        return 1.0 / (2.0 * EPSILON * rate.sqrt());
    }
}
