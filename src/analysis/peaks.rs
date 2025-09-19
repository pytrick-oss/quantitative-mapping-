use crate::analysis::density::DensityAnalysis;

#[derive(Debug, Clone)]
pub struct DensityPeak {
    pub price: f64,
    pub density: f64,
    pub prominence: f64,
}

pub fn detect_peaks(density: &DensityAnalysis) -> Vec<DensityPeak> {
    if density.grid.len() < 3 {
        return Vec::new();
    }

    let mut peaks = Vec::new();
    for i in 1..density.grid.len() - 1 {
        let prev = &density.grid[i - 1];
        let curr = &density.grid[i];
        let next = &density.grid[i + 1];

        if curr.density <= prev.density || curr.density <= next.density {
            continue;
        }

        let left_base = prev.density.min(curr.density);
        let right_base = next.density.min(curr.density);
        let prominence = curr.density - left_base.min(right_base);
        peaks.push(DensityPeak {
            price: curr.price,
            density: curr.density,
            prominence,
        });
    }

    peaks.sort_by(|a, b| {
        b.prominence
            .partial_cmp(&a.prominence)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    peaks
}
