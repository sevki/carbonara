/// Converts Gigabytes to kWh
///
/// # Arguments
///
/// * `gigabytes` - The amount of data in Gigabytes
///
/// # Returns
///
/// * The energy consumption in kWh
pub fn gigabytes_to_kwh(gigabytes: f64) -> f64 {
    gigabytes * 0.0028125
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
pub fn megabytes_to_kwh(megabytes: f64) -> f64 {
    megabytes * 0.0000028125
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
pub fn kwh_to_co2e(kwh: f64, co2e_per_kwh: f64) -> f64 {
    kwh * co2e_per_kwh
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
pub fn joules_to_kwh(joules: f64) -> f64 {
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
pub fn kwh_to_joules(kwh: f64) -> f64 {
    kwh * 3_600_000.0
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
pub fn tdp_to_joules(tdp: f64, time_seconds: f64) -> f64 {
    tdp * time_seconds
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
pub fn benchmarks_to_kwh(runtime_seconds: f64, average_power_watts: f64) -> f64 {
    (runtime_seconds * average_power_watts) / 3_600_000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gigabytes_to_kwh() {
        assert_eq!(gigabytes_to_kwh(1.0), 0.0028125);
    }

    #[test]
    fn test_megabytes_to_kwh() {
        assert_eq!(megabytes_to_kwh(1.0), 0.0000028125);
    }

    #[test]
    fn test_kwh_to_co2e() {
        assert_eq!(kwh_to_co2e(1.0, 436.0), 436.0);
    }

    #[test]
    fn test_joules_to_kwh() {
        assert_eq!(joules_to_kwh(3_600_000.0), 1.0);
    }

    #[test]
    fn test_kwh_to_joules() {
        assert_eq!(kwh_to_joules(1.0), 3_600_000.0);
    }

    #[test]
    fn test_tdp_to_joules() {
        assert_eq!(tdp_to_joules(28.0, 5.0), 140.0);
    }

    #[test]
    fn test_benchmarks_to_kwh() {
        assert_eq!(benchmarks_to_kwh(3600.0, 100.0), 0.1);
    }
}
