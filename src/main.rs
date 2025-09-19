mod analysis;
mod config;
mod data;
mod loader;
mod output;

use std::path::Path;

use analysis::{
    auto_dbscan_epsilon, build_levels, cluster_swings, compute_atr, compute_density_curve,
    detect_peaks, detect_swings, evaluate_levels, ClusterResult, DensityAnalysis,
};
use anyhow::{bail, Context, Result};
use chrono_tz::America::New_York;
use clap::Parser;

use config::AppConfig;
use data::{Level, RthWindow};
use loader::{filter_rth, load_bars_from_csv, validate_series};
use output::{print_report, AthContext};

fn main() -> Result<()> {
    let config = AppConfig::parse();
    run(&config)
}

fn run(config: &AppConfig) -> Result<()> {
    let input_path = &config.input_path;
    if !Path::new(input_path).exists() {
        bail!("input file {:?} does not exist", input_path);
    }

    let raw_bars = load_bars_from_csv(input_path)
        .with_context(|| format!("failed to load input data from {:?}", input_path))?;
    validate_series(&raw_bars)?;

    let rth = RthWindow::default();
    let bars = filter_rth(&raw_bars, rth);
    if bars.is_empty() {
        bail!("no bars remain after applying the regular trading hours filter");
    }
    validate_series(&bars)?;

    let start = bars.first().unwrap();
    let end = bars.last().unwrap();
    println!(
        "Loaded {} RTH bars spanning {} to {} (Eastern)",
        bars.len(),
        start
            .timestamp
            .with_timezone(&New_York)
            .format("%Y-%m-%d %H:%M"),
        end.timestamp
            .with_timezone(&New_York)
            .format("%Y-%m-%d %H:%M")
    );

    let atr = compute_atr(&bars, config.atr_period);
    let mean_atr = if atr.is_empty() {
        0.0
    } else {
        atr.iter().copied().sum::<f64>() / atr.len() as f64
    };

    let swings = detect_swings(
        &bars,
        &atr,
        config.atr_multiplier,
        config.min_swing_distance,
    );
    if swings.len() < config.dbscan_min_points {
        bail!(
            "insufficient swing points ({}) for clustering",
            swings.len()
        );
    }
    println!("Detected {} swing points", swings.len());

    let base_eps = auto_dbscan_epsilon(&swings);
    let epsilon = if base_eps > 0.0 {
        base_eps * config.dbscan_eps_factor
    } else {
        mean_atr.max(config.min_swing_distance)
    };

    let ClusterResult { clusters, inliers } =
        cluster_swings(&swings, epsilon, config.dbscan_min_points);
    let clustered_swings = if !inliers.is_empty() {
        inliers
    } else {
        swings.clone()
    };
    println!(
        "Formed {} price clusters (ε = {:.4}); retained {} swing observations",
        clusters.len(),
        epsilon,
        clustered_swings.len()
    );

    let density: DensityAnalysis = compute_density_curve(&clustered_swings, config.kde_points);
    if density.is_empty() {
        bail!("density estimation failed; not enough clustered swing data");
    }

    let peaks = detect_peaks(&density);
    if peaks.is_empty() {
        bail!("no significant density peaks detected");
    }

    let current_price = end.close;
    let mut levels: Vec<Level> = build_levels(
        &peaks,
        density.max_density,
        current_price,
        mean_atr,
        config.confidence_band_atr,
        config.max_levels,
    );

    levels = evaluate_levels(
        levels,
        &bars,
        &atr,
        config.reaction_lookahead,
        config.reaction_move_atr,
    );

    let ath = bars
        .iter()
        .max_by(|a, b| {
            a.high
                .partial_cmp(&b.high)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|bar| AthContext {
            price: bar.high,
            timestamp: bar.timestamp,
        });

    print_report(&levels, current_price, ath, &density);

    Ok(())
}
