use crate::data::{Bar, SwingPoint, SwingType};

/// Detect swing highs and lows using an ATR-governed zig-zag algorithm.
pub fn detect_swings(
    bars: &[Bar],
    atr: &[f64],
    atr_multiplier: f64,
    min_swing_distance: f64,
) -> Vec<SwingPoint> {
    if bars.is_empty() {
        return Vec::new();
    }

    let mut swings = Vec::new();
    let mut last_type = Some(SwingType::Low);
    let mut last_index = 0usize;
    let mut last_price = bars[0].low;
    let initial_atr = atr.get(0).copied().unwrap_or(0.0);
    swings.push(SwingPoint {
        index: 0,
        bar: bars[0].clone(),
        price: bars[0].low,
        swing_type: SwingType::Low,
        atr: initial_atr,
    });

    let mut candidate_high_price = bars[0].high;
    let mut candidate_high_idx = 0usize;
    let mut candidate_low_price = bars[0].low;
    let mut candidate_low_idx = 0usize;

    for idx in 1..bars.len() {
        let bar = &bars[idx];
        let atr_val = atr.get(idx).copied().unwrap_or(initial_atr);
        let threshold = (atr_val * atr_multiplier)
            .abs()
            .max(min_swing_distance)
            .max(1e-6);

        if bar.high >= candidate_high_price {
            candidate_high_price = bar.high;
            candidate_high_idx = idx;
        }
        if bar.low <= candidate_low_price {
            candidate_low_price = bar.low;
            candidate_low_idx = idx;
        }

        match last_type {
            Some(SwingType::Low) | None => {
                if candidate_high_price - last_price >= threshold && candidate_high_idx > last_index
                {
                    let pivot_bar = bars[candidate_high_idx].clone();
                    push_swing(
                        &mut swings,
                        SwingPoint {
                            index: candidate_high_idx,
                            bar: pivot_bar.clone(),
                            price: pivot_bar.high,
                            swing_type: SwingType::High,
                            atr: atr.get(candidate_high_idx).copied().unwrap_or(atr_val),
                        },
                    );
                    last_type = Some(SwingType::High);
                    last_index = candidate_high_idx;
                    last_price = pivot_bar.high;
                    candidate_low_idx = candidate_high_idx;
                    candidate_low_price = pivot_bar.low;
                }
            }
            Some(SwingType::High) => {
                if last_price - candidate_low_price >= threshold && candidate_low_idx > last_index {
                    let pivot_bar = bars[candidate_low_idx].clone();
                    push_swing(
                        &mut swings,
                        SwingPoint {
                            index: candidate_low_idx,
                            bar: pivot_bar.clone(),
                            price: pivot_bar.low,
                            swing_type: SwingType::Low,
                            atr: atr.get(candidate_low_idx).copied().unwrap_or(atr_val),
                        },
                    );
                    last_type = Some(SwingType::Low);
                    last_index = candidate_low_idx;
                    last_price = pivot_bar.low;
                    candidate_high_idx = candidate_low_idx;
                    candidate_high_price = pivot_bar.high;
                }
            }
        }
    }

    swings.sort_by_key(|s| s.index);
    swings.dedup_by(|a, b| a.index == b.index && a.swing_type == b.swing_type);
    swings
}

fn push_swing(swings: &mut Vec<SwingPoint>, swing: SwingPoint) {
    if let Some(last) = swings.last() {
        if last.index == swing.index && last.swing_type == swing.swing_type {
            return;
        }
    }
    swings.push(swing);
}
