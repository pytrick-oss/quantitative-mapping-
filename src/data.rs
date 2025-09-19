use chrono::{DateTime, NaiveTime};
use chrono_tz::Tz;
use serde::Serialize;

/// Single OHLCV bar sampled at a uniform interval.
#[derive(Debug, Clone, Serialize)]
pub struct Bar {
    pub timestamp: DateTime<Tz>,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum SwingType {
    High,
    Low,
}

/// Extracted price swing with context.
#[derive(Debug, Clone, Serialize)]
pub struct SwingPoint {
    pub index: usize,
    pub bar: Bar,
    pub price: f64,
    pub swing_type: SwingType,
    pub atr: f64,
}

/// Cluster of similar swing prices.
#[derive(Debug, Clone, Serialize)]
pub struct PriceCluster {
    pub id: usize,
    pub representative_price: f64,
    pub total_volume: f64,
    pub swing_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct DensityPoint {
    pub price: f64,
    pub density: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum LevelType {
    Support,
    Resistance,
}

#[derive(Debug, Clone, Serialize)]
pub struct PerformanceStats {
    pub touches: usize,
    pub tests: usize,
    pub hit_rate: f64,
    pub avg_reaction: f64,
    pub max_favorable_excursion: f64,
    pub avg_reaction_bars: f64,
}

impl PerformanceStats {
    pub fn empty() -> Self {
        Self {
            touches: 0,
            tests: 0,
            hit_rate: 0.0,
            avg_reaction: 0.0,
            max_favorable_excursion: 0.0,
            avg_reaction_bars: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Level {
    pub price: f64,
    pub density: f64,
    pub confidence: f64,
    pub confidence_band: f64,
    pub level_type: LevelType,
    pub performance: PerformanceStats,
    pub distance_from_last: f64,
}

/// Utility describing the regular trading hours window in Eastern time.
#[derive(Debug, Clone, Copy)]
pub struct RthWindow {
    pub start: NaiveTime,
    pub end: NaiveTime,
}

impl Default for RthWindow {
    fn default() -> Self {
        Self {
            start: NaiveTime::from_hms_opt(9, 30, 0).unwrap(),
            end: NaiveTime::from_hms_opt(16, 0, 0).unwrap(),
        }
    }
}

impl RthWindow {
    pub fn contains(&self, timestamp: &DateTime<Tz>) -> bool {
        let time = timestamp.time();
        time >= self.start && time < self.end
    }
}
