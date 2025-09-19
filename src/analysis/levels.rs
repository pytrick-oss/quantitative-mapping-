use crate::analysis::peaks::DensityPeak;
use crate::data::{Level, LevelType, PerformanceStats};

pub fn build_levels(
    peaks: &[DensityPeak],
    max_density: f64,
    current_price: f64,
    mean_atr: f64,
    confidence_band_multiplier: f64,
    max_levels: usize,
) -> Vec<Level> {
    if peaks.is_empty() {
        return Vec::new();
    }

    let max_prominence = peaks
        .iter()
        .map(|peak| peak.prominence)
        .fold(0.0, f64::max)
        .max(1e-9);

    let mut levels: Vec<Level> = peaks
        .iter()
        .map(|peak| {
            let density_score = if max_density > 0.0 {
                (peak.density / max_density).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let prominence_score = (peak.prominence / max_prominence).clamp(0.0, 1.0);
            let confidence = 0.6 * density_score + 0.4 * prominence_score;
            let base_band = mean_atr * confidence_band_multiplier;
            let confidence_band = if base_band > 0.0 {
                base_band
            } else {
                (peak.price.abs() * 0.001).max(0.25)
            };
            let level_type = if peak.price >= current_price {
                LevelType::Resistance
            } else {
                LevelType::Support
            };
            Level {
                price: peak.price,
                density: peak.density,
                confidence,
                confidence_band,
                level_type,
                performance: PerformanceStats::empty(),
                distance_from_last: (peak.price - current_price).abs(),
            }
        })
        .collect();

    levels.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    levels.truncate(max_levels);
    levels
}
