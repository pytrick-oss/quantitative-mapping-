use chrono::DateTime;
use chrono_tz::Tz;
use tabled::{settings::Style, Table, Tabled};

use crate::analysis::density::DensityAnalysis;
use crate::data::{Level, LevelType};

pub struct AthContext {
    pub price: f64,
    pub timestamp: DateTime<Tz>,
}

#[derive(Tabled)]
struct LevelRow {
    #[tabled(rename = "Type")]
    kind: &'static str,
    #[tabled(rename = "Price")]
    price: String,
    #[tabled(rename = "Confidence")]
    confidence: String,
    #[tabled(rename = "Band")]
    band: String,
    #[tabled(rename = "Hit Rate")]
    hit_rate: String,
    #[tabled(rename = "Touches")]
    touches: String,
    #[tabled(rename = "Avg React")]
    avg_reaction: String,
    #[tabled(rename = "Max Move")]
    max_move: String,
    #[tabled(rename = "Bars")]
    bars: String,
}

pub fn print_report(
    levels: &[Level],
    current_price: f64,
    ath: Option<AthContext>,
    density: &DensityAnalysis,
) {
    println!("\n=== Quantitative Level Recon ===\n");
    println!("Current Price: {current_price:.2}");
    if let Some(ath) = ath {
        println!(
            "All-Time High: {price:.2} (set {timestamp})",
            price = ath.price,
            timestamp = ath.timestamp.format("%Y-%m-%d %H:%M"),
        );
    }

    if !density.is_empty() {
        if let (Some(first), Some(last)) = (density.grid.first(), density.grid.last()) {
            println!(
                "Density Grid: {} points | Peak density {:.4}",
                density.grid.len(),
                density.max_density
            );
            let bandwidth_info = if density.bandwidths.is_empty() {
                "auto".to_string()
            } else {
                density
                    .bandwidths
                    .iter()
                    .map(|bw| format!("{bw:.4}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            println!("Bandwidths: {bandwidth_info}");
            println!("Price Range: {:.2} to {:.2}", first.price, last.price);
        }
    }

    if levels.is_empty() {
        println!("No statistically meaningful levels identified.");
        return;
    }

    let rows: Vec<LevelRow> = levels
        .iter()
        .map(|level| {
            let tests = level.performance.tests;
            let hit_rate = if tests > 0 {
                format!("{:.1}%", level.performance.hit_rate * 100.0)
            } else {
                "-".to_string()
            };
            let touches = if tests > 0 {
                format!("{}", tests)
            } else {
                "-".to_string()
            };
            let avg_reaction = if tests > 0 {
                format!("{:.2}", level.performance.avg_reaction)
            } else {
                "-".to_string()
            };
            let max_move = if tests > 0 {
                format!("{:.2}", level.performance.max_favorable_excursion)
            } else {
                "-".to_string()
            };
            let bars = if tests > 0 && level.performance.avg_reaction_bars > 0.0 {
                format!("{:.1}", level.performance.avg_reaction_bars)
            } else {
                "-".to_string()
            };
            LevelRow {
                kind: match level.level_type {
                    LevelType::Support => "Support",
                    LevelType::Resistance => "Resistance",
                },
                price: format!("{:.2}", level.price),
                confidence: format!("{:.2}", level.confidence * 100.0),
                band: format!("+/-{:.2}", level.confidence_band),
                hit_rate,
                touches,
                avg_reaction,
                max_move,
                bars,
            }
        })
        .collect();

    let mut table = Table::new(rows);
    table.with(Style::rounded());
    println!("\n{table}\n");
}
