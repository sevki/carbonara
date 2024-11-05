#![deny(missing_docs)]
#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/sevki/carbonara/main/carbonara.png")]
#![doc(html_favicon_url = "https://raw.githubusercontent.com/sevki/carbonara/main/carbonara.png")]

use std::{
    fmt::Display,
    fs::{self, File},
    io::{self, BufRead, BufReader},
    path::Path,
    str::FromStr,
    thread,
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use uom::si::f64::Energy;
use uom::si::{energy::joule, f64::Power};
use uom::si::{energy::kilowatt_hour, power::watt};

/// Converts Gigabytes to kWh
///
/// # Arguments
///
/// * `gigabytes` - The amount of data in Gigabytes
///
/// # Returns
///
/// * The energy consumption in kWh
pub fn gigabytes_to_kwh(gigabytes: f64) -> Energy {
    Energy::new::<kilowatt_hour>(gigabytes * 0.0028125)
}

/// Converts Megabytes to kWh
///
/// # Arguments
///
/// * `megabytes` - The amount of data in Megabytes
///
/// # Returns
///
/// * The energy consumption in kWh
pub fn megabytes_to_kwh(megabytes: f64) -> Energy {
    Energy::new::<kilowatt_hour>(megabytes * 0.0000028125)
}

/// Converts kWh to CO2e
///
/// # Arguments
///
/// * `kwh` - The energy consumption in kWh
/// * `co2e_per_kwh` - The CO2e per kWh (e.g., 436 gCO2e/kWh for global average)
///
/// # Returns
///
/// * The CO2e emissions in grams
pub fn kwh_to_co2e(kwh: Energy, co2e_per_kwh: f64) -> f64 {
    kwh.get::<kilowatt_hour>() * co2e_per_kwh
}

/// Converts Joules to kWh
///
/// # Arguments
///
/// * `joules` - The energy in Joules
///
/// # Returns
///
/// * The energy consumption in kWh
pub fn joules_to_kwh(joules: Energy) -> Energy {
    joules / 3_600_000.0
}

/// Converts kWh to Joules
///
/// # Arguments
///
/// * `kwh` - The energy consumption in kWh
///
/// # Returns
///
/// * The energy in Joules
pub fn kwh_to_joules(kwh: Energy) -> Energy {
    Energy::new::<joule>(kwh.get::<kilowatt_hour>() * 3_600_000.0)
}

/// Estimates energy consumption from TDP (Thermal Design Power)
///
/// # Arguments
///
/// * `tdp` - The Thermal Design Power in Watts
/// * `time_seconds` - The time of usage in seconds
///
/// # Returns
///
/// * The estimated energy consumption in Joules
pub fn tdp_to_joules(tdp: f64, time_seconds: f64) -> Energy {
    Energy::new::<joule>(tdp * time_seconds)
}

/// Estimates energy consumption from benchmarks
///
/// # Arguments
///
/// * `runtime_seconds` - The runtime in seconds
/// * `average_power_watts` - The average power consumption in Watts
///
/// # Returns
///
/// * The estimated energy consumption in kWh
pub fn benchmarks_to_kwh(runtime_seconds: f64, average_power_watts: f64) -> Energy {
    Energy::new::<kilowatt_hour>((runtime_seconds * average_power_watts) / 3_600_000.0)
}

/// Represents different power measurement methods
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
pub enum PowerSource {
    /// Automatically select the best power source
    #[default]
    Auto,
    /// Intel RAPL (Running Average Power Limit)
    Rapl,
    /// System-wide power consumption via ACPI
    Acpi,
    /// TDP-based estimation (least accurate)
    TdpEstimate,
}

impl Display for PowerSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PowerSource::Auto => write!(f, "Auto"),
            PowerSource::Rapl => write!(f, "RAPL"),
            PowerSource::Acpi => write!(f, "ACPI"),
            PowerSource::TdpEstimate => write!(f, "TDP Estimate"),
        }
    }
}

impl FromStr for PowerSource {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "auto" => Ok(PowerSource::Auto),
            "rapl" => Ok(PowerSource::Rapl),
            "acpi" => Ok(PowerSource::Acpi),
            "tdp" => Ok(PowerSource::TdpEstimate),
            _ => Err(format!("Unknown power source: {}", s)),
        }
    }
}

/// Measurement configuration
#[derive(Debug)]
pub struct MeasurementConfig {
    /// Duration of the measurement
    pub duration: Duration,
    /// Preferred power source for measurements
    pub power_source: PowerSource,
    /// Sample interval in milliseconds
    pub sample_interval_ms: u64,
}

/// Measurement results
#[derive(Debug, Serialize, Deserialize)]
pub struct EnergyMeasurement {
    /// Total energy consumed
    pub total_energy: Energy,
    /// Average power
    pub average_power: Power,
    /// Peak power observed
    pub peak_power: Power,
    /// Duration of measurement
    pub duration: Duration,
    /// Method used for measurement
    pub measurement_method: PowerSource,
}

impl Display for EnergyMeasurement {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Total energy: {:.2} kWh\nAverage power: {:.2} W\nPeak power: {:.2} W\nDuration: {:?}\nMethod: {:?}",
            self.total_energy.get::<kilowatt_hour>(),
            self.average_power.get::<watt>(),
            self.peak_power.get::<watt>(),
            self.duration,
            self.measurement_method
        )
    }
}

impl EnergyMeasurement {
    /// Converts the total energy consumed to kWh
    pub fn co2e(&self, co2e_per_kwh: Option<f64>) -> f64 {
        kwh_to_co2e(
            joules_to_kwh(self.total_energy),
            co2e_per_kwh.unwrap_or(436.0),
        )
    }
}

/// ACPI power supply information
#[derive(Debug)]
struct AcpiPowerInfo {
    voltage_now: f64,        // μV
    current_now: f64,        // μA
    power_now: Option<f64>,  // μW
    energy_now: Option<f64>, // μWh
}

/// ACPI measurement implementation
pub struct AcpiMeasurement {
    power_supply_path: String,
    cached_power_supplies: Vec<String>,
}

impl AcpiMeasurement {
    /// Creates a new ACPI measurement instance
    pub fn new() -> Result<Self, MeasurementError> {
        let base_path = "/sys/class/power_supply";
        if !Path::new(base_path).exists() {
            return Err(MeasurementError::AcpiNotAvailable);
        }

        // Find available power supplies
        let entries = fs::read_dir(base_path).map_err(|_| MeasurementError::AcpiNotAvailable)?;

        let mut power_supplies = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let Some(name) = path.file_name() else {
                    continue;
                };
                let Some(name_str) = name.to_str() else {
                    continue;
                };
                // Only include BAT* and AC* devices
                if name_str.starts_with("BAT") || name_str.starts_with("AC") {
                    power_supplies.push(name_str.to_string());
                }
            }
        }

        if power_supplies.is_empty() {
            return Err(MeasurementError::AcpiNotAvailable);
        }

        Ok(Self {
            power_supply_path: base_path.to_string(),
            cached_power_supplies: power_supplies,
        })
    }

    fn read_power_info(&self) -> Result<Vec<AcpiPowerInfo>, MeasurementError> {
        let mut results = Vec::new();

        for supply in &self.cached_power_supplies {
            let base_path = format!("{}/{}", self.power_supply_path, supply);

            // Helper function to read numeric value from ACPI file
            let read_value = |filename: &str| -> Result<Option<f64>, MeasurementError> {
                let path = format!("{}/{}", base_path, filename);
                if !Path::new(&path).exists() {
                    return Ok(None);
                }

                let content = fs::read_to_string(&path).map_err(MeasurementError::IoError)?;
                let value = content.trim().parse::<f64>().map_err(|_| {
                    MeasurementError::InvalidMeasurement(format!(
                        "Failed to parse {} for {}",
                        filename, supply
                    ))
                })?;
                Ok(Some(value))
            };

            // Read available metrics
            let voltage = read_value("voltage_now")?.unwrap_or(0.0);
            let current = read_value("current_now")?.unwrap_or(0.0);
            let power = read_value("power_now")?;
            let energy = read_value("energy_now")?;

            results.push(AcpiPowerInfo {
                voltage_now: voltage,
                current_now: current,
                power_now: power,
                energy_now: energy,
            });
        }

        Ok(results)
    }

    fn calculate_power(&self, info: &[AcpiPowerInfo]) -> f64 {
        let mut total_power = 0.0;

        for supply in info {
            // If power_now is available, use it directly
            if let Some(power) = supply.power_now {
                total_power += power;
            } else {
                // Otherwise calculate from voltage and current
                total_power += (supply.voltage_now * supply.current_now) / 1_000_000.0;
                // Convert to Watts
            }
        }

        total_power / 1_000_000.0 // Convert μW to W
    }
}

#[derive(Debug)]
/// Measurement errors
pub enum MeasurementError {
    /// I/O error
    IoError(io::Error),
    /// RAPL not available
    RaplNotAvailable,
    /// ACPI not available
    AcpiNotAvailable,
    /// Invalid measurement data
    InvalidMeasurement(String),
}
impl From<io::Error> for MeasurementError {
    fn from(error: io::Error) -> Self {
        MeasurementError::IoError(error)
    }
}

/// Intel RAPL measurement implementation
pub struct RaplMeasurement {
    package_path: String,
}

impl RaplMeasurement {
    /// Create a new RAPL measurement instance
    pub fn new() -> Result<Self, MeasurementError> {
        // Check if RAPL is available
        let package_path = "/sys/class/powercap/intel-rapl/intel-rapl:0/energy_uj";
        if !std::path::Path::new(package_path).exists() {
            return Err(MeasurementError::RaplNotAvailable);
        }
        // check if we have permission to read the file
        if File::open(package_path).is_err() {
            return Err(MeasurementError::RaplNotAvailable);
        };
        Ok(Self {
            package_path: package_path.to_string(),
        })
    }

    fn read_energy_counter(&self) -> Result<u64, MeasurementError> {
        let file = File::open(&self.package_path)?;
        let mut reader = BufReader::new(file);
        let mut value = String::new();
        reader.read_line(&mut value)?;
        value
            .trim()
            .parse::<u64>()
            .map_err(|e| MeasurementError::InvalidMeasurement(e.to_string()))
    }
}

/// Benchmark executor
pub struct BenchmarkExecutor {
    config: MeasurementConfig,
}

impl BenchmarkExecutor {
    /// Create a new benchmark executor
    pub fn new(config: MeasurementConfig) -> Self {
        Self { config }
    }

    /// Measure energy consumption of a given workload
    pub fn measure<F>(&self, workload: F) -> Result<EnergyMeasurement, MeasurementError>
    where
        F: FnOnce() + Send + 'static,
    {
        match self.config.power_source {
            PowerSource::Auto => {
                // Try RAPL first
                if RaplMeasurement::new().is_ok() {
                    return self.measure_with_rapl(workload);
                }

                // Try ACPI next
                if AcpiMeasurement::new().is_ok() {
                    return self.measure_with_acpi(workload);
                }

                // Fall back to TDP estimate
                self.measure_with_tdp(workload)
            }
            PowerSource::Rapl => self.measure_with_rapl(workload),
            PowerSource::Acpi => self.measure_with_acpi(workload),
            PowerSource::TdpEstimate => self.measure_with_tdp(workload),
        }
    }

    fn measure_with_rapl<F>(&self, workload: F) -> Result<EnergyMeasurement, MeasurementError>
    where
        F: FnOnce() + Send + 'static,
    {
        let rapl = RaplMeasurement::new()?;

        // Initial reading
        let start_energy = rapl.read_energy_counter()?;
        let start_time = Instant::now();

        // Execute workload
        workload();

        // Final reading
        let end_energy = rapl.read_energy_counter()?;
        let duration = start_time.elapsed();

        // Convert microjoules to joules
        let energy_joules = (end_energy - start_energy) as f64 / 1_000_000.0;
        let average_power_watts = energy_joules / duration.as_secs_f64();

        let total_energy: Energy = Energy::new::<joule>(energy_joules);

        let samples = [0.0];

        let peak_power = samples.iter().cloned().fold(0.0, f64::max);

        let peak_power = Power::new::<watt>(peak_power);

        let average_power = Power::new::<watt>(average_power_watts);

        Ok(EnergyMeasurement {
            duration,
            measurement_method: PowerSource::Rapl,
            total_energy,
            average_power,
            peak_power,
        })
    }

    fn measure_with_acpi<F>(&self, workload: F) -> Result<EnergyMeasurement, MeasurementError>
    where
        F: FnOnce() + Send + 'static,
    {
        let acpi = AcpiMeasurement::new()?;

        // Initial reading
        let start_time = Instant::now();
        let mut samples = Vec::new();
        let mut peak_power = 0.0;

        // Spawn sampling thread
        let sample_interval = Duration::from_millis(self.config.sample_interval_ms);
        let duration = self.config.duration;
        let sampling_thread = thread::spawn(move || {
            let mut local_samples = Vec::new();
            let mut local_peak: f64 = 0.0;

            while start_time.elapsed() < duration {
                if let Ok(info) = acpi.read_power_info() {
                    let power = acpi.calculate_power(&info);
                    local_samples.push(power);
                    local_peak = local_peak.max(power);
                }
                thread::sleep(sample_interval);
            }

            (local_samples, local_peak)
        });

        // Execute workload
        workload();

        // Collect measurements
        if let Ok((local_samples, local_peak)) = sampling_thread.join() {
            samples = local_samples;
            peak_power = local_peak;
        }

        let duration = start_time.elapsed();

        // Calculate average power and total energy
        let average_power = if !samples.is_empty() {
            samples.iter().sum::<f64>() / samples.len() as f64
        } else {
            0.0
        };

        let total_energy = average_power * duration.as_secs_f64();
        let total_energy = Energy::new::<joule>(total_energy);

        let average_power = Power::new::<watt>(average_power);
        let peak_power = Power::new::<watt>(peak_power);

        Ok(EnergyMeasurement {
            total_energy,
            average_power,
            peak_power,
            duration,
            measurement_method: PowerSource::Acpi,
        })
    }

    fn measure_with_tdp<F>(&self, workload: F) -> Result<EnergyMeasurement, MeasurementError>
    where
        F: FnOnce() + Send + 'static,
    {
        let start_time = Instant::now();

        // Execute workload
        workload();

        let duration = start_time.elapsed();

        // Estimate using a conservative TDP value (example: 28W for laptop CPU)
        let estimated_tdp = 28.0; // This should be configurable
        let energy_joules = estimated_tdp * duration.as_secs_f64();

        let total_energy = Energy::new::<joule>(energy_joules);
        let average_power = Power::new::<watt>(estimated_tdp);
        let peak_power = Power::new::<watt>(estimated_tdp);

        Ok(EnergyMeasurement {
            total_energy,
            average_power,
            peak_power,
            duration,
            measurement_method: PowerSource::TdpEstimate,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gigabytes_to_kwh() {
        assert_eq!(
            gigabytes_to_kwh(1.0),
            Energy::new::<kilowatt_hour>(0.0028125)
        );
    }

    #[test]
    fn test_megabytes_to_kwh() {
        assert_eq!(
            megabytes_to_kwh(1.0),
            Energy::new::<kilowatt_hour>(0.0000028125)
        );
    }

    #[test]
    fn test_kwh_to_co2e() {
        assert_eq!(kwh_to_co2e(Energy::new::<kilowatt_hour>(1.0), 436.0), 436.0);
    }

    #[test]
    fn test_joules_to_kwh() {
        assert_eq!(joules_to_kwh(Energy::new::<joule>(3_600_000.0)).value, 1.0);
    }

    #[test]
    fn test_kwh_to_joules() {
        assert_eq!(
            kwh_to_joules(Energy::new::<kilowatt_hour>(1.0)).value,
            3_600_000.0
        );
    }

    #[test]
    fn test_tdp_to_joules() {
        assert_eq!(tdp_to_joules(28.0, 5.0).value, 140.0);
    }

    #[test]
    fn test_benchmarks_to_kwh() {
        assert_eq!(benchmarks_to_kwh(3600.0, 100.0).value, 360000.0)
    }

    #[test]
    fn test_tdp_measurement() {
        let config = MeasurementConfig {
            duration: Duration::from_secs(1),
            power_source: PowerSource::TdpEstimate,
            sample_interval_ms: 100,
        };

        let executor = BenchmarkExecutor::new(config);
        let result = executor.measure(|| {
            // Simple CPU-intensive workload
            thread::sleep(Duration::from_secs(1));
        });

        assert!(result.is_ok());
        let measurement = result.unwrap();
        assert!(measurement.total_energy > Energy::new::<joule>(0.0));
        assert_eq!(
            measurement.measurement_method as i32,
            PowerSource::TdpEstimate as i32
        );
    }

    #[test]
    fn test_rapl_availability() {
        let rapl_result = RaplMeasurement::new();
        // This test will pass either way, we just want to know if RAPL is available
        match rapl_result {
            Ok(_) => println!("RAPL is available on this system"),
            Err(MeasurementError::RaplNotAvailable) => {
                println!("RAPL is not available on this system")
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }

    #[test]
    fn test_acpi_availability() {
        let acpi_result = AcpiMeasurement::new();
        match acpi_result {
            Ok(_) => println!("ACPI power measurement is available"),
            Err(MeasurementError::AcpiNotAvailable) => {
                println!("ACPI power measurement is not available on this system")
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }

    #[test]
    fn test_acpi_measurement() {
        let config = MeasurementConfig {
            duration: Duration::from_secs(2),
            power_source: PowerSource::Acpi,
            sample_interval_ms: 100,
        };

        let executor = BenchmarkExecutor::new(config);
        let result = executor.measure(|| {
            // CPU-intensive workload
            for _ in 0..1_000_000 {
                let _ = (0..100).sum::<i32>();
            }
        });

        match result {
            Ok(measurement) => {
                println!("ACPI Measurement Results:");
                println!("Total Energy: {:?}", measurement.total_energy);
                println!("Average Power: {:?}", measurement.average_power);
                println!("Peak Power: {:?}", measurement.peak_power);
                println!("Duration: {:?}", measurement.duration);
            }
            Err(MeasurementError::AcpiNotAvailable) => {
                println!("ACPI measurements not available on this system")
            }
            Err(e) => panic!("Unexpected error during ACPI measurement: {:?}", e),
        }
    }
}
