use crate::data::{PriceCluster, SwingPoint};

#[derive(Debug, Clone)]
pub struct ClusterResult {
    pub clusters: Vec<PriceCluster>,
    pub inliers: Vec<SwingPoint>,
}

/// Estimate a suitable DBSCAN epsilon by examining the swing-price spacing.
pub fn auto_dbscan_epsilon(swings: &[SwingPoint]) -> f64 {
    if swings.len() < 2 {
        return 0.0;
    }
    let mut prices: Vec<f64> = swings.iter().map(|s| s.price).collect();
    prices.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let mut diffs = Vec::with_capacity(prices.len().saturating_sub(1));
    for window in prices.windows(2) {
        diffs.push((window[1] - window[0]).abs());
    }

    let median = median(&diffs).unwrap_or(0.0);
    if median.is_finite() && median > 0.0 {
        median
    } else {
        let price_scale = prices.last().copied().unwrap_or(1.0).abs().max(1.0);
        0.001 * price_scale
    }
}

pub fn cluster_swings(swings: &[SwingPoint], epsilon: f64, min_points: usize) -> ClusterResult {
    if swings.is_empty() {
        return ClusterResult {
            clusters: Vec::new(),
            inliers: Vec::new(),
        };
    }
    if epsilon <= 0.0 || !epsilon.is_finite() {
        return ClusterResult {
            clusters: Vec::new(),
            inliers: Vec::new(),
        };
    }

    let mut sorted: Vec<(usize, &SwingPoint)> = swings.iter().enumerate().collect();
    sorted.sort_by(|a, b| {
        a.1.price
            .partial_cmp(&b.1.price)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut clusters = Vec::new();
    let mut inliers = Vec::new();
    let mut buffer: Vec<(usize, &SwingPoint)> = Vec::new();

    for &(idx, swing) in &sorted {
        if buffer.is_empty() {
            buffer.push((idx, swing));
            continue;
        }

        let last_price = buffer.last().unwrap().1.price;
        if (swing.price - last_price).abs() <= epsilon {
            buffer.push((idx, swing));
        } else {
            maybe_emit_cluster(&mut clusters, &mut inliers, &mut buffer, min_points);
            buffer.clear();
            buffer.push((idx, swing));
        }
    }
    maybe_emit_cluster(&mut clusters, &mut inliers, &mut buffer, min_points);

    ClusterResult { clusters, inliers }
}

fn maybe_emit_cluster(
    clusters: &mut Vec<PriceCluster>,
    inliers: &mut Vec<SwingPoint>,
    buffer: &mut Vec<(usize, &SwingPoint)>,
    min_points: usize,
) {
    if buffer.len() < min_points {
        return;
    }
    let id = clusters.len();
    let total_volume: f64 = buffer.iter().map(|(_, s)| s.bar.volume).sum();
    let representative_price = if total_volume > 0.0 {
        buffer
            .iter()
            .map(|(_, s)| s.price * s.bar.volume)
            .sum::<f64>()
            / total_volume
    } else {
        buffer.iter().map(|(_, s)| s.price).sum::<f64>() / buffer.len() as f64
    };

    clusters.push(PriceCluster {
        id,
        representative_price,
        total_volume,
        swing_count: buffer.len(),
    });
    for (_, swing) in buffer.iter() {
        inliers.push((*swing).clone());
    }
}

fn median(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        Some(0.5 * (sorted[mid - 1] + sorted[mid]))
    } else {
        Some(sorted[mid])
    }
}
