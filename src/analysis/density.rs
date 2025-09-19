use std::f64::consts::PI;

use crate::data::{DensityPoint, SwingPoint};

#[derive(Debug, Clone)]
pub struct DensityAnalysis {
    pub grid: Vec<DensityPoint>,
    pub bandwidths: Vec<f64>,
    pub max_density: f64,
}

impl DensityAnalysis {
    pub fn is_empty(&self) -> bool {
        self.grid.is_empty()
    }
}

pub fn compute_density_curve(swings: &[SwingPoint], grid_points: usize) -> DensityAnalysis {
    if swings.is_empty() || grid_points < 3 {
        return DensityAnalysis {
            grid: Vec::new(),
            bandwidths: Vec::new(),
            max_density: 0.0,
        };
    }

    let mut min_price = f64::MAX;
    let mut max_price = f64::MIN;
    let mut weights = Vec::with_capacity(swings.len());
    let mut prices = Vec::with_capacity(swings.len());
    for swing in swings {
        let price = swing.price;
        if !price.is_finite() {
            continue;
        }
        let weight = swing.bar.volume.max(1.0);
        min_price = min_price.min(price);
        max_price = max_price.max(price);
        weights.push(weight);
        prices.push(price);
    }

    if prices.len() < 2 {
        return DensityAnalysis {
            grid: Vec::new(),
            bandwidths: Vec::new(),
            max_density: 0.0,
        };
    }

    let total_weight: f64 = weights.iter().sum();
    let mean = prices
        .iter()
        .zip(weights.iter())
        .map(|(price, weight)| price * weight)
        .sum::<f64>()
        / total_weight;
    let variance = prices
        .iter()
        .zip(weights.iter())
        .map(|(price, weight)| weight * (price - mean).powi(2))
        .sum::<f64>()
        / total_weight;
    let std_dev = variance.sqrt().max(1e-6);
    let n = prices.len() as f64;
    let base_bandwidth = 1.06 * std_dev * n.powf(-0.2);

    let mut bandwidths = vec![base_bandwidth * 0.75, base_bandwidth, base_bandwidth * 1.5];
    bandwidths.retain(|bw| bw.is_finite() && *bw > 0.0);
    bandwidths.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    bandwidths.dedup_by(|a, b| (*a - *b).abs() < 1e-6);

    let margin = (max_price - min_price).abs().max(std_dev) * 0.15;
    min_price -= margin;
    max_price += margin;
    let step = if grid_points > 1 {
        (max_price - min_price) / (grid_points - 1) as f64
    } else {
        0.0
    };

    let mut grid = Vec::with_capacity(grid_points);
    let mut max_density: f64 = 0.0;

    let valid_bandwidths = bandwidths.len().max(1) as f64;
    for idx in 0..grid_points {
        let price = min_price + step * idx as f64;
        let mut density = 0.0;
        for &bandwidth in &bandwidths {
            density += gaussian_kernel_sum(price, &prices, &weights, bandwidth, total_weight);
        }
        if bandwidths.is_empty() {
            density = gaussian_kernel_sum(price, &prices, &weights, std_dev, total_weight);
        } else {
            density /= valid_bandwidths;
        }
        max_density = max_density.max(density);
        grid.push(DensityPoint { price, density });
    }

    DensityAnalysis {
        grid,
        bandwidths,
        max_density,
    }
}

fn gaussian_kernel_sum(
    price: f64,
    points: &[f64],
    weights: &[f64],
    bandwidth: f64,
    total_weight: f64,
) -> f64 {
    if bandwidth <= 0.0 {
        return 0.0;
    }
    let norm = 1.0 / (total_weight * bandwidth * (2.0 * PI).sqrt());
    let mut sum = 0.0;
    for (point, weight) in points.iter().zip(weights.iter()) {
        let z = (price - point) / bandwidth;
        sum += weight * (-0.5 * z * z).exp();
    }
    norm * sum
}
