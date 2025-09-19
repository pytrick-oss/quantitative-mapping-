use crate::data::Bar;

/// Compute an exponential (Wilder) Average True Range series.
pub fn compute_atr(bars: &[Bar], period: usize) -> Vec<f64> {
    if bars.is_empty() || period == 0 {
        return Vec::new();
    }

    let mut true_ranges = Vec::with_capacity(bars.len());
    for (idx, bar) in bars.iter().enumerate() {
        let tr = if idx == 0 {
            bar.high - bar.low
        } else {
            let prev = &bars[idx - 1];
            let high_low = bar.high - bar.low;
            let high_close = (bar.high - prev.close).abs();
            let low_close = (bar.low - prev.close).abs();
            high_low.max(high_close).max(low_close)
        };
        true_ranges.push(tr.max(0.0));
    }

    if true_ranges.len() < period {
        let avg = true_ranges.iter().copied().sum::<f64>() / true_ranges.len() as f64;
        return vec![avg; bars.len()];
    }

    let mut atr_values = vec![0.0; bars.len()];
    let initial = true_ranges[..period].iter().copied().sum::<f64>() / period as f64;
    atr_values[period - 1] = initial;

    let mut prev_atr = initial;
    for idx in period..true_ranges.len() {
        let tr = true_ranges[idx];
        prev_atr = (prev_atr * (period as f64 - 1.0) + tr) / period as f64;
        atr_values[idx] = prev_atr;
    }

    for idx in 0..period - 1 {
        atr_values[idx] = atr_values[period - 1];
    }

    atr_values
}
