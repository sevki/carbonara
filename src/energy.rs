use argh::FromArgs;
use carbonara::{
    BenchmarkExecutor, EnergyMeasurement, MeasurementConfig, MeasurementError, PowerSource,
};
use okstd::prelude::*;
use std::{convert::Infallible, fmt::Display, process::Command, str::FromStr, time::Duration};
use uom::si::{
    energy::{joule, kilowatt_hour},
    power::watt,
    Unit,
};

enum Format {
    Human,
    Json,
    Csv,
}

impl Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Format::Human => write!(f, "human"),
            Format::Json => write!(f, "json"),
            Format::Csv => write!(f, "csv"),
        }
    }
}

impl FromStr for Format {
    type Err = Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "human" => Ok(Format::Human),
            "json" => Ok(Format::Json),
            "csv" => Ok(Format::Csv),
            _ => unreachable!(),
        }
    }
}

#[derive(FromArgs)]
/// A CLI tool like `time` but for energy consumption.
struct EnergyTool {
    /// measurement method to use (rapl, acpi, tdp)
    #[argh(option, short = 'm', default = "PowerSource::Acpi")]
    method: PowerSource,

    /// sampling interval in milliseconds
    #[argh(option, short = 'i', default = "100")]
    interval: u64,

    /// output format (human, json, csv)
    #[argh(option, short = 'f', default = "Format::Human")]
    format: Format,

    /// duration to measure for in milliseconds
    #[argh(option, short = 'd', default = "1000")]
    duration: u64,

    /// co2e_per_kwh - The CO2e per kWh (e.g., 436 gCO2e/kWh for global average)
    #[argh(option, short = 'c', default = "436.0")]
    co2e_per_kwh: f64,

    /// the command to run and measure
    #[argh(positional)]
    command: Vec<String>,
}

async fn measure_command(
    command: Vec<String>,
    config: MeasurementConfig,
) -> Result<EnergyMeasurement, MeasurementError> {
    let exec = BenchmarkExecutor::new(config);
    exec.measure(move || {
        let mut cmd = Command::new(&command[0]);
        for arg in command.iter().skip(1) {
            cmd.arg(arg);
        }
        cmd.status().expect("failed to execute process");
    })
}

fn format_measurement(
    measurement: &EnergyMeasurement,
    format: Format,
    co2e_per_kwh: f64,
) -> String {
    match format {
        Format::Human => format!(
            "Energy Measurement Results:\n\
             Energy consumed: {:.2} {}  ({:.2} {})\n\
             Average power: {:.2} {} \n\
             Peak power: {:.2} {}\n\
             Duration: {:.2} {}\n\
             CO2e: {:.2} {}\n\
             Measurement method: {}",
            measurement.total_energy.get::<kilowatt_hour>(), uom::si::energy::kilowatt_hour::plural(),
            measurement.total_energy.get::<joule>(), uom::si::energy::joule::plural(),
            measurement.average_power.get::<watt>(), uom::si::power::watt::plural(),
            measurement.peak_power.get::<watt>(), uom::si::power::watt::plural(),
            measurement.duration.as_secs(), uom::si::time::second::plural(),
            measurement.co2e(Some(co2e_per_kwh)), uom::si::mass::gram::plural(),
            measurement.measurement_method,


        ),

        Format::Json => serde_json::to_string_pretty(&measurement).unwrap(),

        Format::Csv => format!(
            "energy_joules,energy_kwh,power_watts,peak_power_watts,duration_seconds,co2e_grams,measurement_method\n\
             {},{},{},{},{},{},{}",
            measurement.total_energy.get::<joule>(),
            measurement.total_energy.get::<kilowatt_hour>(),
            measurement.average_power.get::<watt>(),
            measurement.peak_power.get::<watt>(),
            measurement.duration.as_secs(),
            measurement.co2e(Some(co2e_per_kwh)),
            measurement.measurement_method,

        ),
    }
}

#[okstd::main]
async fn main() {
    let args: EnergyTool = argh::from_env();

    if args.command.is_empty() {
        eprintln!("No command provided");
        std::process::exit(1);
    }

    let config = MeasurementConfig {
        power_source: args.method,
        duration: Duration::from_millis(args.duration),
        sample_interval_ms: args.interval,
    };

    match measure_command(args.command, config).await {
        Ok(result) => {
            println!(
                "{}",
                format_measurement(&result, args.format, args.co2e_per_kwh)
            );
        }
        Err(e) => {
            eprintln!("Error measuring command: {:?}", e);
            std::process::exit(1);
        }
    }
}
