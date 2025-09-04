use std::{fmt::Display, iter::Sum, ops::Add, str::FromStr};

#[cfg(test)]
#[path = "./metrics.tests.rs"]
mod metrics_tests;

/// Possible errors from parsing kubernetes metrics.
#[derive(thiserror::Error, Debug)]
pub enum MetricsError {
    /// Failed to parse specified metrics.
    #[error("failed to parse specified metrics")]
    ParseError,
}

const KB: u64 = 1_000;
const KIB: u64 = 1_024;
const MB: u64 = KB * 1_000;
const MIB: u64 = KIB * 1_024;
const GB: u64 = MB * 1_000;
const GIB: u64 = MIB * 1_024;
const TB: u64 = GB * 1_000;
const TIB: u64 = GIB * 1_024;
const PB: u64 = TB * 1_000;
const PIB: u64 = TIB * 1_024;
const EB: u64 = PB * 1_000;
const EIB: u64 = PIB * 1_024;
const DECIMAL_BASE: [u64; 6] = [EB, PB, TB, GB, MB, KB];
const DECIMAL_STR: [&str; 6] = ["EB", "PB", "TB", "GB", "MB", "KB"];
const BINARY_BASE: [u64; 6] = [EIB, PIB, TIB, GIB, MIB, KIB];
const BINARY_STR: [&str; 6] = ["Ei", "Pi", "Ti", "Gi", "Mi", "Ki"];

/// Memory usage metrics.
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct MemoryMetrics {
    pub value: u64,
    pub is_binary: bool,
}

impl MemoryMetrics {
    /// Creates new [`MemoryMetrics`] instance.
    pub fn new(value: u64, is_binary: bool) -> Self {
        Self { value, is_binary }
    }
}

impl Add for MemoryMetrics {
    type Output = MemoryMetrics;

    fn add(self, rhs: Self) -> Self::Output {
        MemoryMetrics::new(self.value + rhs.value, self.is_binary)
    }
}

impl Sum for MemoryMetrics {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(MemoryMetrics::new(0, false), |acc, item| MemoryMetrics {
            value: acc.value + item.value,
            is_binary: item.is_binary,
        })
    }
}

impl Display for MemoryMetrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_binary {
            fmt_memory(f, self.value, &BINARY_BASE, &BINARY_STR)
        } else {
            fmt_memory(f, self.value, &DECIMAL_BASE, &DECIMAL_STR)
        }
    }
}

impl FromStr for MemoryMetrics {
    type Err = MetricsError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let Ok((value, unit)) = split_unit(s) else {
            return Err(MetricsError::ParseError);
        };

        match unit.to_ascii_lowercase().as_str() {
            "" | "b" => Ok(MemoryMetrics::new(value, false)),
            "kb" => Ok(MemoryMetrics::new(value * KB, false)),
            "ki" | "kib" => Ok(MemoryMetrics::new(value * KIB, true)),
            "mb" => Ok(MemoryMetrics::new(value * MB, false)),
            "mi" | "mib" => Ok(MemoryMetrics::new(value * MIB, true)),
            "gb" => Ok(MemoryMetrics::new(value * GB, false)),
            "gi" | "gib" => Ok(MemoryMetrics::new(value * GIB, true)),
            "tb" => Ok(MemoryMetrics::new(value * TB, false)),
            "ti" | "tib" => Ok(MemoryMetrics::new(value * TIB, true)),
            "pb" => Ok(MemoryMetrics::new(value * PB, false)),
            "pi" | "pib" => Ok(MemoryMetrics::new(value * PIB, true)),
            "eb" => Ok(MemoryMetrics::new(value * EB, false)),
            "ei" | "eib" => Ok(MemoryMetrics::new(value * EIB, true)),

            _ => Err(MetricsError::ParseError),
        }
    }
}

/// CPU usage metrics.
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct CpuMetrics {
    pub value: u64,
}

impl CpuMetrics {
    /// Creates new [`CpuMetrics`] instance.
    pub fn new(value: u64) -> Self {
        Self { value }
    }
}

impl Add for CpuMetrics {
    type Output = CpuMetrics;

    fn add(self, rhs: Self) -> Self::Output {
        CpuMetrics::new(self.value + rhs.value)
    }
}

impl Sum for CpuMetrics {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(CpuMetrics::new(0), |acc, item| CpuMetrics {
            value: acc.value + item.value,
        })
    }
}

impl Display for CpuMetrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}n", self.value))
    }
}

impl FromStr for CpuMetrics {
    type Err = MetricsError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let Ok((value, unit)) = split_unit(s) else {
            return Err(MetricsError::ParseError);
        };

        match unit.to_ascii_lowercase().as_str() {
            "" | "n" => Ok(CpuMetrics::new(value)),
            _ => Err(MetricsError::ParseError),
        }
    }
}

/// Memory and CPU usage metrics.
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct Metrics {
    pub memory: MemoryMetrics,
    pub cpu: CpuMetrics,
}

impl Add for Metrics {
    type Output = Metrics;

    fn add(self, rhs: Self) -> Self::Output {
        Metrics {
            memory: self.memory + rhs.memory,
            cpu: self.cpu + rhs.cpu,
        }
    }
}

impl Sum for Metrics {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Metrics::default(), |acc, item| Metrics {
            memory: acc.memory + item.memory,
            cpu: acc.cpu + item.cpu,
        })
    }
}

fn split_unit(input: &str) -> Result<(u64, &str), MetricsError> {
    let index = input.find(|c: char| !c.is_ascii_digit()).unwrap_or(input.len());
    let (value, unit) = input.split_at(index);
    if value.is_empty() {
        return Err(MetricsError::ParseError);
    }

    let Ok(value) = value.parse::<u64>() else {
        return Err(MetricsError::ParseError);
    };

    Ok((value, unit.trim()))
}

fn fmt_memory(f: &mut std::fmt::Formatter<'_>, value: u64, base: &[u64; 6], units: &[&str; 6]) -> std::fmt::Result {
    if value > KB {
        for (i, b) in base.iter().enumerate() {
            if value % b == 0 {
                return f.write_fmt(format_args!("{}{}", value / b, units[i]));
            }
        }
    }

    f.write_fmt(format_args!("{}B", value))
}
