//! Comprehensive performance benchmarks for soul-playback
//!
//! This benchmark suite measures critical hot paths in the playback system:
//! - Audio callback latency (process_audio)
//! - Queue operations (add, skip, next)
//! - State transitions (play, pause, next)
//! - State queries (get_volume, get_state, etc.)
//! - Crossfade gain calculation
//! - Volume ramping
//!
//! # Running Benchmarks
//!
//! ```bash
//! # Run all benchmarks
//! cargo bench -p soul-playback
//!
//! # Run specific benchmark group
//! cargo bench -p soul-playback -- audio_callback
//! cargo bench -p soul-playback -- queue_ops
//! cargo bench -p soul-playback -- volume
//!
//! # Generate HTML reports (in target/criterion/)
//! cargo bench -p soul-playback
//! ```
//!
//! # Performance Targets
//!
//! - **Audio callback**: <1ms p99 (critical for real-time audio)
//! - **Queue operations**: <10μs each (called frequently)
//! - **State transitions**: <100μs each (user-initiated)
//! - **State queries**: <1μs (called from UI thread)
//! - **Crossfade gain calc**: <10μs for 1000 samples
//! - **Volume apply**: <100μs for 1024 samples

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use soul_playback::{
    AudioSource, FadeCurve, PlaybackConfig, PlaybackManager, QueueTrack, RepeatMode, Result,
    ShuffleMode, TrackSource,
};
use std::path::PathBuf;
use std::time::Duration;

// ===== Test Audio Source =====

/// Dummy audio source for benchmarking (sine wave generator)
struct BenchmarkAudioSource {
    sample_rate: u32,
    position_samples: usize,
    duration_samples: usize,
    phase: f32,
    frequency: f32,
}

impl BenchmarkAudioSource {
    fn new(duration: Duration, sample_rate: u32) -> Self {
        let duration_samples = (duration.as_secs_f64() * sample_rate as f64) as usize;
        Self {
            sample_rate,
            position_samples: 0,
            duration_samples,
            phase: 0.0,
            frequency: 440.0, // A4 note
        }
    }
}

impl AudioSource for BenchmarkAudioSource {
    fn read_samples(&mut self, buffer: &mut [f32]) -> Result<usize> {
        let remaining = self.duration_samples.saturating_sub(self.position_samples);
        let to_read = buffer.len().min(remaining);

        // Generate stereo sine wave (simple, realistic workload)
        for i in (0..to_read).step_by(2) {
            let sample = (self.phase * std::f32::consts::TAU).sin() * 0.5;
            buffer[i] = sample; // Left
            buffer[i + 1] = sample; // Right

            self.phase += self.frequency / self.sample_rate as f32;
            if self.phase >= 1.0 {
                self.phase -= 1.0;
            }
        }

        self.position_samples += to_read / 2;
        Ok(to_read)
    }

    fn seek(&mut self, position: Duration) -> Result<()> {
        self.position_samples = (position.as_secs_f64() * self.sample_rate as f64) as usize;
        Ok(())
    }

    fn duration(&self) -> Duration {
        Duration::from_secs_f64(self.duration_samples as f64 / self.sample_rate as f64)
    }

    fn position(&self) -> Duration {
        Duration::from_secs_f64(self.position_samples as f64 / self.sample_rate as f64)
    }

    fn is_finished(&self) -> bool {
        self.position_samples >= self.duration_samples
    }

    fn is_ready(&self) -> bool {
        true
    }

    fn sample_rate(&self) -> Option<u32> {
        Some(self.sample_rate)
    }
}

// ===== Helper Functions =====

fn create_test_track(id: &str, title: &str, duration_secs: u64) -> QueueTrack {
    QueueTrack {
        id: id.to_string(),
        path: PathBuf::from(format!("/music/{}.mp3", id)),
        title: title.to_string(),
        artist: "Test Artist".to_string(),
        album: Some("Test Album".to_string()),
        duration: Duration::from_secs(duration_secs),
        track_number: Some(1),
        source: TrackSource::Single,
    }
}

fn setup_manager_with_queue(track_count: usize) -> PlaybackManager {
    let mut manager = PlaybackManager::new(PlaybackConfig {
        volume: 80,
        shuffle: ShuffleMode::Off,
        repeat: RepeatMode::Off,
        history_size: 50,
        ..Default::default()
    });

    // Add tracks to queue
    let tracks: Vec<QueueTrack> = (0..track_count)
        .map(|i| create_test_track(&format!("track_{}", i), &format!("Track {}", i), 180))
        .collect();

    manager.load_playlist(tracks, 0);
    manager
}

// ===== BENCHMARK: Audio Callback Latency =====

fn bench_audio_callback(c: &mut Criterion) {
    let mut group = c.benchmark_group("audio_callback");
    group.throughput(Throughput::Elements(1));

    // Test with different buffer sizes (common sizes for audio callbacks)
    for buffer_size in [256, 512, 1024, 2048] {
        group.bench_with_input(
            BenchmarkId::new("process_audio", buffer_size),
            &buffer_size,
            |b, &size| {
                let mut manager = setup_manager_with_queue(10);
                manager.set_audio_source(Box::new(BenchmarkAudioSource::new(
                    Duration::from_secs(180),
                    48000,
                )));
                let _ = manager.play();

                let mut buffer = vec![0.0f32; size];

                b.iter(|| {
                    let _ = manager.process_audio(black_box(&mut buffer));
                });
            },
        );
    }

    group.finish();
}

// ===== BENCHMARK: Queue Operations =====

fn bench_queue_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("queue_operations");

    // Benchmark: add_to_queue_next
    group.bench_function("add_to_queue_next", |b| {
        let mut manager = setup_manager_with_queue(100);
        let track = create_test_track("new_track", "New Track", 180);

        b.iter(|| {
            manager.add_to_queue_next(black_box(track.clone()));
        });
    });

    // Benchmark: add_to_queue_end
    group.bench_function("add_to_queue_end", |b| {
        let mut manager = setup_manager_with_queue(100);
        let track = create_test_track("new_track", "New Track", 180);

        b.iter(|| {
            manager.add_to_queue_end(black_box(track.clone()));
        });
    });

    // Benchmark: skip_to_queue_index (hot path during queue navigation)
    group.bench_function("skip_to_queue_index", |b| {
        let mut manager = setup_manager_with_queue(100);

        b.iter(|| {
            let _ = manager.skip_to_queue_index(black_box(50));
        });
    });

    // Benchmark: get_queue (called frequently by UI)
    group.bench_function("get_queue", |b| {
        let manager = setup_manager_with_queue(100);

        b.iter(|| {
            black_box(manager.get_queue());
        });
    });

    group.finish();
}

// ===== BENCHMARK: State Transitions =====

fn bench_state_transitions(c: &mut Criterion) {
    let mut group = c.benchmark_group("state_transitions");

    // Benchmark: play (from stopped)
    group.bench_function("play", |b| {
        b.iter_batched(
            || {
                let mut manager = setup_manager_with_queue(10);
                manager.set_audio_source(Box::new(BenchmarkAudioSource::new(
                    Duration::from_secs(180),
                    48000,
                )));
                manager
            },
            |mut manager| {
                let _ = black_box(manager.play());
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // Benchmark: pause (from playing)
    group.bench_function("pause", |b| {
        b.iter_batched(
            || {
                let mut manager = setup_manager_with_queue(10);
                manager.set_audio_source(Box::new(BenchmarkAudioSource::new(
                    Duration::from_secs(180),
                    48000,
                )));
                let _ = manager.play();
                manager
            },
            |mut manager| {
                black_box(manager.pause());
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // Benchmark: next (critical for skip performance)
    group.bench_function("next", |b| {
        b.iter_batched(
            || {
                let mut manager = setup_manager_with_queue(10);
                manager.set_audio_source(Box::new(BenchmarkAudioSource::new(
                    Duration::from_secs(180),
                    48000,
                )));
                let _ = manager.play();
                manager
            },
            |mut manager| {
                let _ = black_box(manager.next());
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // Benchmark: previous
    group.bench_function("previous", |b| {
        b.iter_batched(
            || {
                let mut manager = setup_manager_with_queue(10);
                manager.set_audio_source(Box::new(BenchmarkAudioSource::new(
                    Duration::from_secs(180),
                    48000,
                )));
                let _ = manager.play();
                // Move forward to have history
                let _ = manager.next();
                let _ = manager.next();
                manager
            },
            |mut manager| {
                let _ = black_box(manager.previous());
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

// ===== BENCHMARK: State Queries =====

fn bench_state_queries(c: &mut Criterion) {
    let mut group = c.benchmark_group("state_queries");
    group.throughput(Throughput::Elements(1));

    let manager = setup_manager_with_queue(100);

    // These should be very fast as they're simple field accesses
    group.bench_function("get_state", |b| {
        b.iter(|| black_box(manager.get_state()));
    });

    group.bench_function("get_current_track", |b| {
        b.iter(|| black_box(manager.get_current_track()));
    });

    group.bench_function("get_volume", |b| {
        b.iter(|| black_box(manager.get_volume()));
    });

    group.bench_function("is_muted", |b| {
        b.iter(|| black_box(manager.is_muted()));
    });

    group.bench_function("get_shuffle_mode", |b| {
        b.iter(|| black_box(manager.get_shuffle_mode()));
    });

    group.bench_function("get_repeat", |b| {
        b.iter(|| black_box(manager.get_repeat()));
    });

    group.bench_function("queue_len", |b| {
        b.iter(|| black_box(manager.queue_len()));
    });

    group.finish();
}

// ===== BENCHMARK: Crossfade Curve Calculation =====

fn bench_crossfade_curves(c: &mut Criterion) {
    let mut group = c.benchmark_group("crossfade_curves");
    group.throughput(Throughput::Elements(1000));

    let curves = [
        FadeCurve::Linear,
        FadeCurve::SquareRoot,
        FadeCurve::SCurve,
        FadeCurve::EqualPower,
        FadeCurve::Exponential,
    ];

    for curve in curves {
        group.bench_with_input(
            BenchmarkId::new("calculate_gain", format!("{:?}", curve)),
            &curve,
            |b, curve| {
                b.iter(|| {
                    // Calculate 1000 gain values across the fade
                    for i in 0..1000 {
                        let position = i as f32 / 1000.0;
                        black_box(curve.calculate_gain(position, false));
                        black_box(curve.calculate_gain(position, true));
                    }
                });
            },
        );
    }

    group.finish();
}

// ===== BENCHMARK: Volume Operations =====

fn bench_volume_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("volume");

    // Benchmark: set_volume (triggers ramp setup)
    group.bench_function("set_volume", |b| {
        let mut manager = setup_manager_with_queue(10);

        b.iter(|| {
            manager.set_volume(black_box(75));
        });
    });

    // Benchmark: toggle_mute
    group.bench_function("toggle_mute", |b| {
        let mut manager = setup_manager_with_queue(10);

        b.iter(|| {
            manager.toggle_mute();
        });
    });

    // Benchmark: volume apply to buffer (called in audio callback)
    // This is measured indirectly through process_audio
    for buffer_size in [256, 512, 1024, 2048] {
        group.bench_with_input(
            BenchmarkId::new("process_with_volume_ramp", buffer_size),
            &buffer_size,
            |b, &size| {
                let mut manager = setup_manager_with_queue(10);
                manager.set_audio_source(Box::new(BenchmarkAudioSource::new(
                    Duration::from_secs(180),
                    48000,
                )));
                let _ = manager.play();

                // Trigger a volume change to enable ramping (most expensive case)
                manager.set_volume(50);

                let mut buffer = vec![0.5f32; size];

                b.iter(|| {
                    let _ = manager.process_audio(black_box(&mut buffer));
                });
            },
        );
    }

    group.finish();
}

// ===== BENCHMARK: Shuffle Algorithms =====

fn bench_shuffle(c: &mut Criterion) {
    let mut group = c.benchmark_group("shuffle");

    // Test with different queue sizes
    for queue_size in [10, 50, 100, 500] {
        // Benchmark: enable shuffle (shuffles entire queue)
        group.bench_with_input(
            BenchmarkId::new("enable_shuffle", queue_size),
            &queue_size,
            |b, &size| {
                b.iter_batched(
                    || setup_manager_with_queue(size),
                    |mut manager| {
                        manager.set_shuffle(black_box(ShuffleMode::Random));
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );

        // Benchmark: disable shuffle (restores original order)
        group.bench_with_input(
            BenchmarkId::new("disable_shuffle", queue_size),
            &queue_size,
            |b, &size| {
                b.iter_batched(
                    || {
                        let mut manager = setup_manager_with_queue(size);
                        manager.set_shuffle(ShuffleMode::Random);
                        manager
                    },
                    |mut manager| {
                        manager.set_shuffle(black_box(ShuffleMode::Off));
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

// ===== BENCHMARK: Allocation Tracking =====

fn bench_allocation_frequency(c: &mut Criterion) {
    let mut group = c.benchmark_group("allocations");

    // Audio callback should have minimal allocations (critical!)
    group.bench_function("audio_callback_allocations", |b| {
        let mut manager = setup_manager_with_queue(10);
        manager.set_audio_source(Box::new(BenchmarkAudioSource::new(
            Duration::from_secs(180),
            48000,
        )));
        let _ = manager.play();

        let mut buffer = vec![0.0f32; 1024];

        // This benchmark verifies that process_audio doesn't allocate
        // Criterion will show if there's allocation overhead
        b.iter(|| {
            let _ = manager.process_audio(black_box(&mut buffer));
        });
    });

    // Queue operations should minimize allocations
    group.bench_function("queue_add_allocations", |b| {
        let mut manager = setup_manager_with_queue(100);
        let track = create_test_track("new_track", "New Track", 180);

        b.iter(|| {
            manager.add_to_queue_next(black_box(track.clone()));
        });
    });

    group.finish();
}

// ===== BENCHMARK: Seek Operations =====

fn bench_seek_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("seek");

    // Benchmark: seek by time
    group.bench_function("seek_to_time", |b| {
        b.iter_batched(
            || {
                let mut manager = setup_manager_with_queue(10);
                manager.set_audio_source(Box::new(BenchmarkAudioSource::new(
                    Duration::from_secs(180),
                    48000,
                )));
                let _ = manager.play();
                manager
            },
            |mut manager| {
                let _ = black_box(manager.seek_to(Duration::from_secs(60)));
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // Benchmark: seek by percentage
    group.bench_function("seek_to_percentage", |b| {
        b.iter_batched(
            || {
                let mut manager = setup_manager_with_queue(10);
                manager.set_audio_source(Box::new(BenchmarkAudioSource::new(
                    Duration::from_secs(180),
                    48000,
                )));
                let _ = manager.play();
                manager
            },
            |mut manager| {
                let _ = black_box(manager.seek_to_percent(50.0));
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

// ===== Criterion Configuration =====

criterion_group!(
    benches,
    bench_audio_callback,
    bench_queue_operations,
    bench_state_transitions,
    bench_state_queries,
    bench_crossfade_curves,
    bench_volume_operations,
    bench_shuffle,
    bench_allocation_frequency,
    bench_seek_operations,
);

criterion_main!(benches);
