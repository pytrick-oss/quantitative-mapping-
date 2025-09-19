pub mod atr;
pub mod clustering;
pub mod density;
pub mod levels;
pub mod peaks;
pub mod stats;
pub mod swings;

pub use atr::compute_atr;
pub use clustering::{auto_dbscan_epsilon, cluster_swings, ClusterResult};
pub use density::{compute_density_curve, DensityAnalysis};
pub use levels::build_levels;
pub use peaks::detect_peaks;
pub use stats::evaluate_levels;
pub use swings::detect_swings;
