//! Soul Player ESP32-S3 DAP Firmware
//!
//! Embedded firmware for a standalone Digital Audio Player (DAP) running on ESP32-S3.
//! Uses Embassy async runtime for efficient task management.

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::{
    clock::ClockControl,
    gpio::{Io, Level, Output},
    peripherals::Peripherals,
    prelude::*,
    timer::timg::TimerGroup,
};
use log::{info, warn};

mod config;
// Module structure (to be implemented as drivers are added)
// mod drivers;
// mod services;
// mod tasks;
// mod state;
// mod boot;

/// Global heap allocator for ESP32-S3
#[global_allocator]
static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

/// Initialize the heap allocator
///
/// Allocates 128KB of heap memory for dynamic allocations.
/// This is sufficient for most operations while leaving plenty
/// of room for stack and static data.
fn init_heap() {
    const HEAP_SIZE: usize = 128 * 1024; // 128KB heap
    static mut HEAP: core::mem::MaybeUninit<[u8; HEAP_SIZE]> = core::mem::MaybeUninit::uninit();

    unsafe {
        ALLOCATOR.init(HEAP.as_mut_ptr() as *mut u8, HEAP_SIZE);
    }
}

/// Main entry point with Embassy executor
///
/// Initializes hardware, sets up the Embassy runtime, and spawns async tasks.
#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    // Initialize heap
    init_heap();
    info!("Heap initialized (128KB)");

    // Take ownership of peripherals
    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();

    // Configure CPU clock to 240MHz for maximum performance
    let clocks = ClockControl::max(system.clock_control).freeze();
    info!("CPU clock: {}MHz", clocks.cpu_clock.to_MHz());

    // Initialize Embassy timer using Timer Group 0
    let timer_group0 = TimerGroup::new(peripherals.TIMG0, &clocks);
    esp_hal_embassy::init(&clocks, timer_group0.timer0);
    info!("Embassy executor initialized");

    // Initialize GPIO subsystem
    let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);

    // Configure built-in LED (GPIO48 on ESP32-S3-DevKitC-1)
    // This serves as a heartbeat indicator during development
    let led = Output::new(io.pins.gpio48, Level::Low);

    info!("Soul Player ESP32-S3 DAP firmware starting...");
    info!("Hardware: ESP32-S3 @ 240MHz");
    info!("Flash: 16MB, PSRAM: 8MB");

    // Spawn the heartbeat task (LED blink)
    spawner
        .spawn(heartbeat_task(led))
        .expect("Failed to spawn heartbeat task");

    // Future tasks to be spawned as they are implemented:
    // spawner.spawn(tasks::audio_task()).unwrap();
    // spawner.spawn(tasks::display_task()).unwrap();
    // spawner.spawn(tasks::input_task()).unwrap();
    // spawner.spawn(tasks::power_task()).unwrap();
    // spawner.spawn(tasks::bluetooth_task()).unwrap();

    info!("All tasks spawned, entering event loop");

    // Main task can perform background work or just idle
    loop {
        Timer::after(Duration::from_secs(10)).await;
        info!("System alive - heap free: {} bytes", heap_free());
    }
}

/// Heartbeat task - blinks the LED to indicate system is running
///
/// Pattern: 100ms ON, 900ms OFF (1Hz blink)
/// This provides visual confirmation that the firmware is running and
/// the Embassy executor is functioning correctly.
#[embassy_executor::task]
async fn heartbeat_task(mut led: Output<'static>) {
    info!("Heartbeat task started");

    loop {
        led.set_high();
        Timer::after(Duration::from_millis(100)).await;

        led.set_low();
        Timer::after(Duration::from_millis(900)).await;
    }
}

/// Get free heap memory in bytes
///
/// Useful for monitoring memory usage during development.
/// In production, consider logging this periodically to detect memory leaks.
fn heap_free() -> usize {
    // ESP heap doesn't expose a direct API for free memory in esp-alloc
    // For now, return 0. This will be implemented properly when we add
    // heap monitoring capabilities.
    0
}

/// Panic handler
///
/// This is already provided by esp-backtrace, but we could add custom
/// crash logging here (e.g., write panic info to NVS for post-mortem analysis).
///
/// For development, the panic message is printed to UART and the system resets.
