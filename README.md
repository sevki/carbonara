![carbonara](https://raw.githubusercontent.com/sevki/carbonara/main/carbonara.png)

# Carbonara is a Rust library for calculating the energy consumption and CO2e emissions of the Internet.
based on [Green Coding Co2 formulas](https://www.green-coding.io/co2-formulas/)

 ```rust
 use carbonara::{MeasurementConfig, BenchmarkExecutor, PowerSource, MeasurementError};
 use std::time::Duration;
 // Example usage with Auto power source detection, which will try RAPL first and then fall back to the ACPI power meter,
 // if that also fails, it will use TDP based power estimation.
 fn main() -> Result<(), MeasurementError> {
     let config = MeasurementConfig {
         duration: Duration::from_secs(5),
         power_source: PowerSource::Auto,
         sample_interval_ms: 100,
     };

     let executor = BenchmarkExecutor::new(config);


     let result = executor.measure(|| {
         // CPU-intensive workload
         for _ in 0..1_000_000 {
             let _ = (0..100).sum::<i32>();
         }
     })?;


     println!("{} Measurement Results:", result.measurement_method);
     println!("Total Energy: {:?}", result.total_energy);
     println!("Average Power: {:?}", result.average_power);
     println!("Peak Power: {:?}", result.peak_power);
     println!("Duration: {:?}", result.duration);
     println!("CO2e: {:.2} g", result.co2e(None));

     Ok(())
 }
 ```
