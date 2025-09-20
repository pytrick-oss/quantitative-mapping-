use std::cmp::Ordering;

use crate::data::{Bar, Level, LevelType, PerformanceStats};

/// Compute EVT-based resistance projections using a peaks-over-threshold model.
pub fn compute_evt_resistances(
    bars: &[Bar],
    tail_probs: &[f64],
    threshold_quantile: f64,
    confidence_band: f64,
    current_price: f64,
) -> Vec<Level> {
    if bars.len() < 50 || tail_probs.is_empty() {
        return Vec::new();
    }

    let mut highs: Vec<f64> = bars.iter().map(|bar| bar.high).collect();
    highs.sort_by(|a, b| match (a.is_finite(), b.is_finite()) {
        (true, true) => a.partial_cmp(b).unwrap_or(Ordering::Equal),
        (false, true) => Ordering::Less,
        (true, false) => Ordering::Greater,
        (false, false) => Ordering::Equal,
    });

    let n = highs.len();
    let threshold_idx = ((n as f64 * threshold_quantile).floor() as usize).clamp(0, n - 1);
    let threshold = highs[threshold_idx];
    let max_high = *highs.last().unwrap_or(&threshold);
    let exceedances: Vec<f64> = highs
        .iter()
        .filter(|&&value| value > threshold)
        .map(|&value| value - threshold)
        .collect();
    let nu = exceedances.len();
    if nu < 5 {
        return Vec::new();
    }

    let lambda = nu as f64 / n as f64;
    if lambda <= 0.0 {
        return Vec::new();
    }

    let mean_excess: f64 = exceedances.iter().sum::<f64>() / nu as f64;
    let variance: f64 = exceedances
        .iter()
        .map(|x| (x - mean_excess).powi(2))
        .sum::<f64>()
        / (nu as f64 - 1.0).max(1.0);

    let (shape, scale) = if variance > 0.0 {
        let ratio = mean_excess * mean_excess / variance;
        let mut xi = 0.5 * (1.0 - ratio);
        if !xi.is_finite() {
            xi = 0.0;
        }
        let mut sigma = mean_excess * (1.0 - xi);
        if !sigma.is_finite() || sigma <= 0.0 {
            xi = 0.0;
            sigma = mean_excess.max(1e-6);
        }
        (xi, sigma)
    } else {
        (0.0, mean_excess.max(1e-6))
    };

    let mut levels = Vec::new();
    for &p in tail_probs {
        if !(0.0..1.0).contains(&p) {
            continue;
        }
        let tail_prob = 1.0 - p;
        if tail_prob <= 0.0 || tail_prob >= lambda {
            continue;
        }
        let ratio = lambda / tail_prob;
        let mut projected = if shape.abs() <= 1e-6 {
            threshold + scale * ratio.ln()
        } else {
            let inner = ratio.powf(shape);
            threshold + (scale / shape) * (inner - 1.0)
        };
        projected = projected.min(max_high + confidence_band.max(1.0) * 5.0);
        if !projected.is_finite() || projected <= threshold || projected <= current_price {
            continue;
        }
        let confidence = p.clamp(0.0, 1.0);
        let mut level = Level {
            price: projected,
            density: 0.0,
            confidence,
            confidence_band,
            level_type: LevelType::Resistance,
            performance: PerformanceStats::empty(),
            distance_from_last: (projected - current_price).abs(),
        };
        if level.confidence_band <= 0.0 {
            level.confidence_band = (projected.abs() * 0.001).max(1.0);
        }
        levels.push(level);
    }

    if levels.is_empty() {
        let step = confidence_band.max(1.0);
        let mut fallback = (max_high + step).min(max_high + step * 5.0);
        if fallback <= current_price {
            fallback = (current_price + step).min(max_high + step * 5.0);
        }
        if fallback.is_finite() {
            let confidence = tail_probs.first().copied().unwrap_or(0.99).clamp(0.0, 1.0);
            let mut level = Level {
                price: fallback,
                density: 0.0,
                confidence,
                confidence_band,
                level_type: LevelType::Resistance,
                performance: PerformanceStats::empty(),
                distance_from_last: (fallback - current_price).abs(),
            };
            if level.confidence_band <= 0.0 {
                level.confidence_band = (fallback.abs() * 0.001).max(1.0);
            }
            levels.push(level);
        }
    }

    levels.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(Ordering::Equal)
    });
    levels
}
