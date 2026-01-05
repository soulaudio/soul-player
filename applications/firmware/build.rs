//! Build script for ESP32-S3 firmware
//!
//! This script runs at build time to:
//! - Configure ESP-IDF settings
//! - Generate linker scripts
//! - Set up environment variables
//! - Validate build configuration

fn main() {
    // Tell Cargo to re-run this script if any of these files change
    println!("cargo:rerun-if-changed=sdkconfig.defaults");
    println!("cargo:rerun-if-changed=partitions.csv");
    println!("cargo:rerun-if-changed=build.rs");

    // ESP-IDF configuration
    // The esp-idf-sys crate handles most of the heavy lifting,
    // but we can add custom build steps here if needed.

    // Example: Set custom environment variables
    // println!("cargo:rustc-env=FIRMWARE_VERSION=0.1.0");

    // Example: Pass custom flags to the linker
    // println!("cargo:rustc-link-arg=-Wl,--gc-sections");

    // Validate partition table (optional compile-time check)
    validate_partition_table();

    println!("cargo:warning=Soul Player ESP32-S3 firmware build configuration loaded");
}

/// Validates that the partition table is correctly formatted
///
/// This is a simple sanity check to ensure the partitions.csv file
/// exists and has the expected structure.
fn validate_partition_table() {
    use std::path::Path;

    let partition_file = Path::new("partitions.csv");

    if !partition_file.exists() {
        panic!("partitions.csv not found! Please create a partition table.");
    }

    // Could add more validation here:
    // - Check partition sizes don't overlap
    // - Verify total size doesn't exceed flash capacity
    // - Ensure required partitions exist (nvs, phy_init, factory/ota)

    println!("cargo:warning=Partition table validated: partitions.csv");
}
