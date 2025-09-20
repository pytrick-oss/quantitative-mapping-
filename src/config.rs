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
    #[arg(long, default_value_t = 0.3)]
    pub atr_multiplier: f64,

    /// Minimum absolute swing distance to register a new swing (price units).
    #[arg(long, default_value_t = 25.0)]
    pub min_swing_distance: f64,

    /// Number of days of data to analyse. Set to 0 to use all available history.
    #[arg(long, default_value_t = 90)]
    pub lookback_days: usize,

    /// Use a multi-window, regime-aware level aggregation.
    #[arg(long, action = ArgAction::SetTrue)]
    pub regime_aware: bool,

    /// Apply a stronger recency weighting when estimating densities.
    #[arg(long, action = ArgAction::SetTrue)]
    pub strong_recency: bool,

    /// Enable EVT-based resistance projection.
    #[arg(long, action = ArgAction::SetTrue)]
    pub evt_resistance: bool,

    /// EVT lookback window (days) used to fit the tail distribution.
    #[arg(long, default_value_t = 30)]
    pub ev_lookback_days: usize,

    /// Upper-tail probability (overall) used for the first EVT projection.
    #[arg(long, default_value_t = 0.99)]
    pub ev_tail_probability: f64,

    /// Quantile (0-1) defining the exceedance threshold for EVT.
    #[arg(long, default_value_t = 0.9)]
    pub ev_threshold_quantile: f64,

    /// Maximum number of EVT resistance levels to generate.
    #[arg(long, default_value_t = 2)]
    pub ev_max_levels: usize,

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
