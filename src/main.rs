mod analysis;
mod config;
mod data;
mod loader;
mod output;

use std::path::Path;

use analysis::{
    auto_dbscan_epsilon, build_levels, cluster_swings, compute_atr, compute_density_curve,
    compute_evt_resistances, detect_peaks, detect_swings, evaluate_levels, ClusterResult,
    DensityAnalysis,
};
use anyhow::{bail, Context, Result};
use chrono::{DateTime, Duration, TimeZone};
use chrono_tz::{America::New_York, Tz};
use clap::Parser;

use config::AppConfig;
use data::{Bar, Level, PerformanceStats, RthWindow, SwingPoint};
use loader::{filter_rth, load_bars_from_csv, validate_series};
use output::{print_report, AthContext};

#[derive(Clone, Copy)]
struct AnalysisSettings {
    recency_half_life_days: Option<f64>,
}

struct AnalysisResult {
    atr: Vec<f64>,
    mean_atr: f64,
    density: DensityAnalysis,
    levels: Vec<Level>,
    swing_count: usize,
}

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

    let base_half_life = if config.strong_recency { 15.0 } else { 30.0 };
    let target_swings = config.dbscan_min_points.max(8);

    let candidate_windows = candidate_lookbacks(config.lookback_days);
    let mut analysis_bars: Vec<Bar> = Vec::new();
    let mut recent_result: Option<AnalysisResult> = None;

    for (idx, window) in candidate_windows.iter().enumerate() {
        let candidate = filter_by_lookback(&bars, *window);
        if candidate.is_empty() {
            continue;
        }

        let label = if *window == 0 {
            "full history".to_string()
        } else {
            format!("last {} days", window)
        };

        if let Some(previous) = &recent_result {
            if previous.swing_count < target_swings {
                println!(
                    "Swing count {} too low; retrying with {} window...",
                    previous.swing_count, label
                );
            }
        }

        print_loaded_summary(&label, &candidate);

        let result = run_single_analysis(
            &candidate,
            config,
            AnalysisSettings {
                recency_half_life_days: Some(base_half_life),
            },
        )?;

        analysis_bars = candidate;
        let done = result.swing_count >= target_swings || idx == candidate_windows.len() - 1;
        recent_result = Some(result);
        if done {
            break;
        }
    }

    if analysis_bars.is_empty() {
        analysis_bars = bars.clone();
    }

    let recent_result = recent_result.unwrap_or_else(|| {
        run_single_analysis(
            &analysis_bars,
            config,
            AnalysisSettings {
                recency_half_life_days: Some(base_half_life),
            },
        )
        .expect("analysis failed")
    });

    if config.regime_aware {
        println!("Running regime-aware aggregation (recent vs. full history)...");
        let historical_result = run_single_analysis(
            &bars,
            config,
            AnalysisSettings {
                recency_half_life_days: Some(base_half_life * 2.0),
            },
        )?;

        let merge_tolerance = recent_result
            .mean_atr
            .max(historical_result.mean_atr)
            .max(config.min_swing_distance)
            .max(2.0);

        let mut combined_levels = combine_level_sets(
            recent_result.levels.clone(),
            historical_result.levels.clone(),
            0.7,
            0.3,
            merge_tolerance,
        );

        let current_price = bars.last().map(|bar| bar.close).unwrap_or_default();
        for level in &mut combined_levels {
            level.distance_from_last = (level.price - current_price).abs();
        }

        let max_slots = config.max_levels + config.ev_max_levels;
        if combined_levels.len() > max_slots {
            combined_levels.truncate(max_slots);
        }

        let evaluated_levels = evaluate_levels(
            combined_levels,
            &bars,
            &historical_result.atr,
            config.reaction_lookahead,
            config.reaction_move_atr,
        );

        let mut final_levels = evaluated_levels;
        if config.evt_resistance {
            let evt_source = if config.ev_lookback_days > 0 {
                filter_by_lookback(&analysis_bars, config.ev_lookback_days)
            } else {
                analysis_bars.clone()
            };
            let tail_probs = build_evt_tail_probs(config.ev_tail_probability, config.ev_max_levels);
            if !tail_probs.is_empty() {
                let mut base_band = recent_result.mean_atr * config.confidence_band_atr;
                if !base_band.is_finite() || base_band <= 0.0 {
                    base_band = (current_price.abs() * 0.001).max(1.0);
                }
                let evt_levels = compute_evt_resistances(
                    &evt_source,
                    &tail_probs,
                    config.ev_threshold_quantile,
                    base_band,
                    current_price,
                );
                if !evt_levels.is_empty() {
                    println!(
                        "EVT projected resistances: {}",
                        evt_levels
                            .iter()
                            .map(|lvl| format!("{:.2}", lvl.price))
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                    final_levels.extend(evt_levels);
                }
            }
        }

        final_levels.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        if final_levels.len() > max_slots {
            final_levels.truncate(max_slots);
        }

        let ath = compute_ath(&bars);
        print_report(&final_levels, current_price, ath, &recent_result.density);
    } else {
        let current_price = analysis_bars
            .last()
            .map(|bar| bar.close)
            .unwrap_or_default();
        let ath = compute_ath(&analysis_bars);

        let mut final_levels = recent_result.levels.clone();
        if config.evt_resistance {
            let evt_source = if config.ev_lookback_days > 0 {
                filter_by_lookback(&analysis_bars, config.ev_lookback_days)
            } else {
                analysis_bars.clone()
            };
            let tail_probs = build_evt_tail_probs(config.ev_tail_probability, config.ev_max_levels);
            if !tail_probs.is_empty() {
                let mut base_band = recent_result.mean_atr * config.confidence_band_atr;
                if !base_band.is_finite() || base_band <= 0.0 {
                    base_band = (current_price.abs() * 0.001).max(1.0);
                }
                let evt_levels = compute_evt_resistances(
                    &evt_source,
                    &tail_probs,
                    config.ev_threshold_quantile,
                    base_band,
                    current_price,
                );
                if !evt_levels.is_empty() {
                    println!(
                        "EVT projected resistances: {}",
                        evt_levels
                            .iter()
                            .map(|lvl| format!("{:.2}", lvl.price))
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                    final_levels.extend(evt_levels);
                }
            }
        }

        final_levels.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let max_slots = config.max_levels + config.ev_max_levels;
        if final_levels.len() > max_slots {
            final_levels.truncate(max_slots);
        }

        print_report(&final_levels, current_price, ath, &recent_result.density);
    }

    Ok(())
}

fn candidate_lookbacks(requested: usize) -> Vec<usize> {
    if requested == 0 {
        vec![0, 90, 60, 45, 30, 20, 15, 10, 5]
    } else {
        let mut windows = vec![requested];
        for alt in [60, 45, 30, 20, 15, 10, 5, 0] {
            if alt != requested {
                windows.push(alt);
            }
        }
        windows
    }
}

fn build_evt_tail_probs(start: f64, max_levels: usize) -> Vec<f64> {
    if max_levels == 0 {
        return Vec::new();
    }
    let mut probs = Vec::with_capacity(max_levels);
    let mut current = start.clamp(0.8, 0.9999);
    while probs.len() < max_levels {
        let capped = current.min(0.9999);
        probs.push(capped);
        if capped >= 0.9999 {
            current = 0.9999;
        } else {
            current += 0.0025;
        }
    }
    probs
}

fn filter_by_lookback(bars: &[Bar], lookback_days: usize) -> Vec<Bar> {
    if bars.is_empty() || lookback_days == 0 {
        return bars.to_vec();
    }
    let cutoff = bars
        .last()
        .map(|bar| bar.timestamp - Duration::days(lookback_days as i64))
        .unwrap_or_else(|| New_York.with_ymd_and_hms(1970, 1, 1, 0, 0, 0).unwrap());
    bars.iter()
        .cloned()
        .filter(|bar| bar.timestamp >= cutoff)
        .collect()
}

fn print_loaded_summary(label: &str, bars: &[Bar]) {
    if let (Some(start), Some(end)) = (bars.first(), bars.last()) {
        println!(
            "Loaded {} RTH bars ({}) spanning {} to {} (Eastern)",
            bars.len(),
            label,
            start
                .timestamp
                .with_timezone(&New_York)
                .format("%Y-%m-%d %H:%M"),
            end.timestamp
                .with_timezone(&New_York)
                .format("%Y-%m-%d %H:%M"),
        );
    }
}

fn apply_recency_weighting(
    swings: &[SwingPoint],
    reference: DateTime<Tz>,
    half_life_days: f64,
) -> Vec<SwingPoint> {
    let mut weighted = swings.to_vec();
    for swing in &mut weighted {
        let age_seconds = (reference - swing.bar.timestamp).num_seconds().max(0) as f64;
        let age_days = age_seconds / 86_400.0;
        let decay = (0.5_f64).powf(age_days / half_life_days.max(1e-6));
        swing.bar.volume = swing.bar.volume.max(1.0) * decay.max(1e-4);
    }
    weighted
}

fn combine_level_sets(
    primary: Vec<Level>,
    secondary: Vec<Level>,
    primary_weight: f64,
    secondary_weight: f64,
    merge_tolerance: f64,
) -> Vec<Level> {
    let mut combined: Vec<Level> = primary
        .into_iter()
        .map(|mut level| {
            level.confidence *= primary_weight;
            level.performance = PerformanceStats::empty();
            level
        })
        .collect();

    for mut level in secondary {
        level.confidence *= secondary_weight;
        level.performance = PerformanceStats::empty();
        let mut merged = false;
        for existing in &mut combined {
            if (existing.price - level.price).abs() <= merge_tolerance {
                let total_conf = existing.confidence + level.confidence;
                if total_conf > 0.0 {
                    existing.price = (existing.price * existing.confidence
                        + level.price * level.confidence)
                        / total_conf;
                    existing.confidence = total_conf;
                    existing.confidence_band =
                        (existing.confidence_band + level.confidence_band) * 0.5;
                }
                merged = true;
                break;
            }
        }
        if !merged {
            combined.push(level);
        }
    }

    combined.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    combined
}

fn compute_ath(bars: &[Bar]) -> Option<AthContext> {
    bars.iter()
        .max_by(|a, b| {
            a.high
                .partial_cmp(&b.high)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|bar| AthContext {
            price: bar.high,
            timestamp: bar.timestamp,
        })
}

fn run_single_analysis(
    bars: &[Bar],
    config: &AppConfig,
    settings: AnalysisSettings,
) -> Result<AnalysisResult> {
    if bars.len() < config.dbscan_min_points {
        bail!("analysis window requires more bars to compute swings");
    }

    let atr = compute_atr(bars, config.atr_period);
    let mean_atr = if atr.is_empty() {
        0.0
    } else {
        atr.iter().copied().sum::<f64>() / atr.len() as f64
    };

    let multiplier_scales = [
        1.0, 0.85, 0.7, 0.55, 0.4, 0.3, 0.25, 0.2, 0.15, 0.1, 0.08, 0.05, 0.03, 0.02, 0.015,
    ];
    let distance_scales = [1.0, 0.75, 0.5, 0.35, 0.25, 0.15, 0.1];
    let min_required_swings = config.dbscan_min_points.max(8);

    let mut swings = Vec::new();
    let mut atr_multiplier_used = config.atr_multiplier;
    let mut min_distance_used = config.min_swing_distance;
    let mut satisfied = false;

    let mut best_swings: Vec<SwingPoint> = Vec::new();
    let mut best_multiplier = config.atr_multiplier;
    let mut best_distance = config.min_swing_distance;

    'outer: for &distance_scale in &distance_scales {
        let candidate_distance = (config.min_swing_distance * distance_scale).max(2.0);
        for &mult_scale in &multiplier_scales {
            let candidate_multiplier = (config.atr_multiplier * mult_scale).max(0.01);
            let candidate_swings =
                detect_swings(bars, &atr, candidate_multiplier, candidate_distance);

            if candidate_swings.len() >= min_required_swings {
                swings = candidate_swings;
                atr_multiplier_used = candidate_multiplier;
                min_distance_used = candidate_distance;
                satisfied = true;
                break 'outer;
            }

            if candidate_swings.len() > best_swings.len() {
                best_swings = candidate_swings;
                best_multiplier = candidate_multiplier;
                best_distance = candidate_distance;
            }
        }
    }

    if !satisfied {
        swings = best_swings;
        atr_multiplier_used = best_multiplier;
        min_distance_used = best_distance;
    }

    if swings.len() < config.dbscan_min_points {
        bail!(
            "insufficient swing points ({}) even after relaxing sensitivity",
            swings.len()
        );
    }

    let swing_count = swings.len();

    if (atr_multiplier_used - config.atr_multiplier).abs() > f64::EPSILON
        || (min_distance_used - config.min_swing_distance).abs() > f64::EPSILON
    {
        println!(
            "Detected {} swing points after relaxing atr_multiplier to {:.3} and min_swing_distance to {:.2}",
            swing_count,
            atr_multiplier_used,
            min_distance_used
        );
    } else {
        println!("Detected {} swing points", swing_count);
    }

    let base_eps = auto_dbscan_epsilon(&swings);
    let epsilon = if base_eps > 0.0 {
        base_eps * config.dbscan_eps_factor
    } else {
        mean_atr.max(min_distance_used).max(1.0)
    };

    let ClusterResult { clusters, inliers } =
        cluster_swings(&swings, epsilon, config.dbscan_min_points);
    let clustered_swings = if !inliers.is_empty() {
        inliers
    } else {
        swings.clone()
    };
    println!(
        "Formed {} price clusters (eps = {:.4}); retained {} swing observations",
        clusters.len(),
        epsilon,
        clustered_swings.len()
    );

    let density_input = if let Some(half_life) = settings.recency_half_life_days {
        let reference = bars
            .last()
            .map(|bar| bar.timestamp)
            .unwrap_or_else(|| New_York.with_ymd_and_hms(1970, 1, 1, 0, 0, 0).unwrap());
        apply_recency_weighting(&clustered_swings, reference, half_life)
    } else {
        clustered_swings.clone()
    };

    let density = compute_density_curve(&density_input, config.kde_points);
    if density.is_empty() {
        bail!("density estimation failed; not enough clustered swing data");
    }

    let peaks = detect_peaks(&density);
    if peaks.is_empty() {
        bail!("no significant density peaks detected");
    }

    let current_price = bars.last().map(|bar| bar.close).unwrap_or_default();
    let mut levels = build_levels(
        &peaks,
        density.max_density,
        current_price,
        mean_atr,
        config.confidence_band_atr,
        config.max_levels + config.ev_max_levels,
    );

    for level in &mut levels {
        level.distance_from_last = (level.price - current_price).abs();
    }

    let levels = evaluate_levels(
        levels,
        bars,
        &atr,
        config.reaction_lookahead,
        config.reaction_move_atr,
    );

    Ok(AnalysisResult {
        atr,
        mean_atr,
        density,
        levels,
        swing_count,
    })
}
