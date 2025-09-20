use std::fs::File;
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
use chrono_tz::{America::New_York, Tz};
use csv::StringRecord;
use thiserror::Error;

use crate::data::{Bar, RthWindow};

#[derive(Debug, Error)]
pub enum LoaderError {
    #[error("input file contains no valid rows")]
    Empty,

    #[error("unable to infer timestamp from record: {0:?}")]
    Timestamp(StringRecord),

    #[error("failed to parse numeric field '{field}' from value '{value}'")]
    ParseNumber { field: &'static str, value: String },
}

pub fn load_bars_from_csv<P: AsRef<Path>>(path: P) -> Result<Vec<Bar>> {
    let path_ref = path.as_ref();
    let file = File::open(path_ref).with_context(|| format!("failed to open {:?}", path_ref))?;
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .trim(csv::Trim::All)
        .flexible(true)
        .from_reader(file);

    let mut bars = Vec::new();
    for record in reader.records() {
        let record = record?;
        if record.iter().all(|field| field.trim().is_empty()) {
            continue;
        }
        match parse_record(&record) {
            Ok(Some(bar)) => bars.push(bar),
            Ok(None) => continue,
            Err(err) => return Err(err),
        }
    }

    if bars.is_empty() {
        return Err(LoaderError::Empty.into());
    }

    bars.sort_by_key(|bar| bar.timestamp);
    Ok(bars)
}

fn parse_record(record: &StringRecord) -> Result<Option<Bar>> {
    // Skip header rows by checking the first field.
    if let Some(first) = record.get(0) {
        if first.trim().eq_ignore_ascii_case("date") {
            return Ok(None);
        }
    }

    let fields: Vec<String> = record
        .iter()
        .map(|f| f.trim().to_string())
        .filter(|f| !f.is_empty())
        .collect();
    if fields.len() < 6 {
        return Ok(None);
    }

    let (datetime, offset) = if fields.len() >= 7 {
        let date = fields[0].as_str();
        let time = fields[1].as_str();
        (parse_datetime_pair(date, time)?, 2)
    } else {
        parse_datetime_string(fields[0].as_str())?
            .map(|dt| (dt, 1))
            .ok_or_else(|| anyhow!(LoaderError::Timestamp(record.clone())))?
    };

    let tz: Tz = New_York;
    let timestamp = match tz.from_local_datetime(&datetime) {
        chrono::LocalResult::Single(dt) => dt,
        chrono::LocalResult::Ambiguous(dt, _) => dt,
        chrono::LocalResult::None => tz.from_utc_datetime(&datetime),
    };

    let open = parse_number(fields.get(offset).map(String::as_str), "open")?;
    let high = parse_number(fields.get(offset + 1).map(String::as_str), "high")?;
    let low = parse_number(fields.get(offset + 2).map(String::as_str), "low")?;
    let close = parse_number(fields.get(offset + 3).map(String::as_str), "close")?;
    let volume = parse_number(fields.get(offset + 4).map(String::as_str), "volume")?;

    Ok(Some(Bar {
        timestamp,
        open,
        high,
        low,
        close,
        volume,
    }))
}

fn parse_number(value: Option<&str>, field: &'static str) -> Result<f64> {
    let value = value.ok_or_else(|| LoaderError::ParseNumber {
        field,
        value: String::from("<missing>"),
    })?;
    value
        .replace(',', "")
        .parse::<f64>()
        .map_err(|_| LoaderError::ParseNumber {
            field,
            value: value.to_string(),
        })
        .map_err(anyhow::Error::from)
}

fn parse_datetime_pair(date_str: &str, time_str: &str) -> Result<NaiveDateTime> {
    let date = parse_date(date_str)?;
    let time = parse_time(time_str)?;
    Ok(NaiveDateTime::new(date, time))
}

fn parse_datetime_string(value: &str) -> Result<Option<NaiveDateTime>> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let patterns = [
        "%Y-%m-%d %H:%M:%S",
        "%Y/%m/%d %H:%M:%S",
        "%m/%d/%Y %H:%M:%S",
        "%Y-%m-%dT%H:%M:%S",
    ];

    for pattern in &patterns {
        if let Ok(datetime) = NaiveDateTime::parse_from_str(trimmed, pattern) {
            return Ok(Some(datetime));
        }
    }

    Ok(None)
}

fn parse_date(value: &str) -> Result<NaiveDate> {
    let patterns = [
        "%Y-%m-%d",
        "%Y-%-m-%-d",
        "%Y-%-m-%d",
        "%Y-%m-%-d",
        "%Y/%m/%d",
        "%Y/%-m/%-d",
        "%Y/%-m/%d",
        "%Y/%m/%-d",
        "%m/%d/%Y",
        "%m/%-d/%Y",
        "%-m/%d/%Y",
        "%-m/%-d/%Y",
    ];
    for pattern in &patterns {
        if let Ok(date) = NaiveDate::parse_from_str(value, pattern) {
            return Ok(date);
        }
    }
    Err(LoaderError::Timestamp(StringRecord::from(vec![value.to_string()])).into())
}

fn parse_time(value: &str) -> Result<NaiveTime> {
    let patterns = ["%H:%M:%S%.f", "%H:%M:%S", "%H:%M"];
    for pattern in &patterns {
        if let Ok(time) = NaiveTime::parse_from_str(value, pattern) {
            return Ok(time);
        }
    }
    Err(LoaderError::Timestamp(StringRecord::from(vec![value.to_string()])).into())
}

pub fn filter_rth(bars: &[Bar], rth: RthWindow) -> Vec<Bar> {
    bars.iter()
        .cloned()
        .filter(|bar| rth.contains(&bar.timestamp))
        .collect()
}

pub fn validate_series(bars: &[Bar]) -> Result<()> {
    if bars.len() < 10 {
        return Err(anyhow!("not enough bars for analysis (need at least 10)"));
    }

    for pair in bars.windows(2) {
        if pair[1].timestamp <= pair[0].timestamp {
            return Err(anyhow!("timestamps must be strictly increasing"));
        }
    }

    Ok(())
}
