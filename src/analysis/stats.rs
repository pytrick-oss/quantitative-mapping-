use crate::data::{Bar, Level, LevelType, PerformanceStats};

pub fn evaluate_levels(
    mut levels: Vec<Level>,
    bars: &[Bar],
    atr: &[f64],
    reaction_lookahead: usize,
    reaction_move_atr: f64,
) -> Vec<Level> {
    if bars.is_empty() {
        return levels;
    }
    let mean_atr = if atr.is_empty() {
        0.0
    } else {
        atr.iter().copied().sum::<f64>() / atr.len() as f64
    };

    for level in &mut levels {
        let mut tests = 0usize;
        let mut hits = 0usize;
        let mut touches = 0usize;
        let mut total_reaction = 0.0;
        let mut max_reaction: f64 = 0.0;
        let mut total_reaction_bars = 0.0;

        for (idx, bar) in bars.iter().enumerate() {
            let tolerance = level.confidence_band;
            let touched = bar.low <= level.price + tolerance && bar.high >= level.price - tolerance;
            if !touched {
                continue;
            }
            tests += 1;
            touches += 1;
            let atr_ref = atr.get(idx).copied().unwrap_or(mean_atr).max(1e-6);

            let mut best_move = 0.0;
            let mut bars_to_best = 0usize;
            let mut success = false;
            let end = (idx + reaction_lookahead + 1).min(bars.len());
            for forward_idx in idx + 1..end {
                let forward_bar = &bars[forward_idx];
                let movement = match level.level_type {
                    LevelType::Support => forward_bar.high - level.price,
                    LevelType::Resistance => level.price - forward_bar.low,
                };
                if movement > best_move {
                    best_move = movement;
                    bars_to_best = forward_idx - idx;
                }
                if movement >= reaction_move_atr * atr_ref {
                    success = true;
                }
            }
            if success {
                hits += 1;
            }
            total_reaction += best_move;
            max_reaction = max_reaction.max(best_move);
            total_reaction_bars += bars_to_best as f64;
        }

        let performance = if tests > 0 {
            PerformanceStats {
                touches,
                tests,
                hit_rate: hits as f64 / tests as f64,
                avg_reaction: total_reaction / tests as f64,
                max_favorable_excursion: max_reaction,
                avg_reaction_bars: total_reaction_bars / tests as f64,
            }
        } else {
            PerformanceStats::empty()
        };

        level.performance = performance;
    }

    levels
}
