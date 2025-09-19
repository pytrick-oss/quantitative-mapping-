use std::path::PathBuf;

use clap::{ArgAction, Parser};

/// Command-line configuration for the quantitative mapping tool.
#[derive(Debug, Clone, Parser)]
#[command(author, version, about, long_about = None)]
pub struct AppConfig {
    /// Input CSV file path containing OHLCV data.
    #[arg(short = 'i', long = "input", value_name = "FILE")]
    pub input_path: String,

    /// ATR period for volatility estimation.
    #[arg(long, default_value_t = 14)]
    pub atr_period: usize,

    /// ATR multiplier controlling swing detection sensitivity.
    #[arg(long, default_value_t = 1.5)]
    pub atr_multiplier: f64,

    /// Minimum absolute swing distance to register a new swing (price units).
    #[arg(long, default_value_t = 0.0)]
    pub min_swing_distance: f64,

    /// KDE grid points for price density estimation.
    #[arg(long, default_value_t = 400)]
    pub kde_points: usize,

    /// DBSCAN epsilon scaling factor (applied to auto-epsilon outcome).
    #[arg(long, default_value_t = 1.0)]
    pub dbscan_eps_factor: f64,

    /// DBSCAN minimum points to form a cluster.
    #[arg(long, default_value_t = 3)]
    pub dbscan_min_points: usize,

    /// Confidence band width in ATR multiples.
    #[arg(long, default_value_t = 1.0)]
    pub confidence_band_atr: f64,

    /// Maximum number of levels to output.
    #[arg(long, default_value_t = 12)]
    pub max_levels: usize,

    /// Lookahead bars for reaction evaluation.
    #[arg(long, default_value_t = 20)]
    pub reaction_lookahead: usize,

    /// Reaction move threshold in ATR multiples.
    #[arg(long, default_value_t = 0.5)]
    pub reaction_move_atr: f64,
}
