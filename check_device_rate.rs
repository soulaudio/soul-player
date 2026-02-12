//! Quick utility to check audio device sample rate and test for resampling

use cpal::traits::{DeviceTrait, HostTrait};

fn main() {
    println!("=== Audio Device Sample Rate Check ===\n");

    let host = cpal::default_host();

    // Get default output device
    let device = host.default_output_device().expect("No default output device");
    println!("Default Output Device: {}", device.name().unwrap_or_else(|_| "Unknown".to_string()));

    // Get default config
    if let Ok(config) = device.default_output_config() {
        println!("\nDefault Configuration:");
        println!("  Sample Rate: {} Hz", config.sample_rate().0);
        println!("  Channels: {}", config.channels());
        println!("  Sample Format: {:?}", config.sample_format());

        let rate = config.sample_rate().0;
        println!("\n=== Analysis ===");

        if rate == 44100 {
            println!("✓ Device is set to 44.1kHz (CD Quality)");
            println!("✓ Your FLAC file (44.1kHz) will play WITHOUT resampling");
            println!("✓ This should sound perfect with no stuttering!");
        } else if rate == 48000 {
            println!("⚠️  Device is set to 48kHz");
            println!("⚠️  Your FLAC file (44.1kHz) REQUIRES resampling");
            println!("⚠️  This is likely causing the stuttering you hear!");
            println!("\n=== Recommended Fix ===");
            println!("Change your audio device to 44.1kHz:");
            println!("1. Right-click sound icon → Sound settings");
            println!("2. Your output device → Properties");
            println!("3. Additional device properties → Advanced tab");
            println!("4. Change to '2 channel, 24 bit, 44100 Hz (CD Quality)'");
        } else {
            println!("⚠️  Device is set to {} Hz", rate);
            println!("⚠️  Your FLAC file (44.1kHz) REQUIRES resampling");
            println!("    Resampling ratio: {:.4}", rate as f64 / 44100.0);
        }
    } else {
        println!("Failed to get default config");
    }

    // List all supported configs
    println!("\n=== Supported Sample Rates ===");
    if let Ok(configs) = device.supported_output_configs() {
        let mut rates = std::collections::HashSet::new();
        for config in configs {
            rates.insert(config.min_sample_rate().0);
            rates.insert(config.max_sample_rate().0);
        }

        let mut sorted: Vec<_> = rates.iter().collect();
        sorted.sort();

        for rate in sorted {
            if *rate == 44100 {
                println!("  {} Hz ✓ (matches your FLAC file)", rate);
            } else if *rate == 48000 {
                println!("  {} Hz (would require resampling)", rate);
            } else {
                println!("  {} Hz", rate);
            }
        }
    }
}
