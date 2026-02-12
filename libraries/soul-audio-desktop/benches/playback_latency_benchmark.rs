//! Playback latency benchmarks
//!
//! Measures critical performance characteristics of the playback system:
//! - Cold start latency (initialization + first audio)
//! - Command processing latency
//! - State query latency
//! - Device enumeration latency

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use soul_audio_desktop::{DesktopPlayback, PlaybackCommand, PlaybackEvent};
use soul_playback::{PlaybackConfig, QueueTrack, TrackSource};
use std::path::PathBuf;
use std::time::Duration;

/// Create a test track with a real audio file path for benchmarking
fn create_test_track(id: &str) -> QueueTrack {
    QueueTrack {
        id: id.to_string(),
        path: PathBuf::from(format!("test_data/track_{}.wav", id)),
        title: format!("Test Track {}", id),
        artist: "Test Artist".to_string(),
        album: Some("Test Album".to_string()),
        duration: Duration::from_secs(180),
        track_number: Some(1),
        source: TrackSource::Single,
    }
}

/// Helper to drain all events from the playback system
fn drain_events(playback: &DesktopPlayback) -> Vec<PlaybackEvent> {
    std::iter::from_fn(|| playback.try_recv_event()).collect()
}

/// Benchmark cold start latency (creation + initialization)
fn bench_cold_start_latency(c: &mut Criterion) {
    c.bench_function("cold_start", |b| {
        b.iter(|| {
            let config = PlaybackConfig::default();
            let playback = DesktopPlayback::new(config).expect("Failed to create playback");
            black_box(playback);
        });
    });
}

/// Benchmark command processing latency
fn bench_command_latency(c: &mut Criterion) {
    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).expect("Failed to create playback");

    let tracks = vec![create_test_track("1")];
    playback
        .send_command(PlaybackCommand::LoadPlaylist(tracks.clone()))
        .expect("Failed to load playlist");

    std::thread::sleep(Duration::from_millis(50));

    let mut group = c.benchmark_group("command_latency");

    group.bench_function("play", |b| {
        b.iter(|| {
            playback
                .send_command(black_box(PlaybackCommand::Play))
                .expect("Failed to send play")
        });
    });

    group.bench_function("pause", |b| {
        b.iter(|| {
            playback
                .send_command(black_box(PlaybackCommand::Pause))
                .expect("Failed to send pause")
        });
    });

    group.bench_function("set_volume", |b| {
        b.iter(|| {
            playback
                .send_command(black_box(PlaybackCommand::SetVolume(0.5)))
                .expect("Failed to set volume")
        });
    });

    group.bench_function("skip_next", |b| {
        b.iter(|| {
            let _ = playback.send_command(black_box(PlaybackCommand::SkipNext));
        });
    });

    group.finish();
}

/// Benchmark event polling latency
fn bench_event_polling_latency(c: &mut Criterion) {
    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).expect("Failed to create playback");

    c.bench_function("event_poll", |b| {
        b.iter(|| {
            let events = drain_events(&playback);
            black_box(events);
        });
    });
}

/// Benchmark playlist loading with different sizes
fn bench_playlist_loading(c: &mut Criterion) {
    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).expect("Failed to create playback");

    let mut group = c.benchmark_group("playlist_loading");

    for size in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let tracks: Vec<_> = (0..size)
                .map(|i| create_test_track(&i.to_string()))
                .collect();

            b.iter(|| {
                playback
                    .send_command(black_box(PlaybackCommand::LoadPlaylist(tracks.clone())))
                    .expect("Failed to load playlist")
            });
        });
    }

    group.finish();
}

/// Benchmark concurrent command throughput
fn bench_concurrent_commands(c: &mut Criterion) {
    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).expect("Failed to create playback");

    let tracks = vec![create_test_track("1"), create_test_track("2")];
    playback
        .send_command(PlaybackCommand::LoadPlaylist(tracks.clone()))
        .expect("Failed to load playlist");

    std::thread::sleep(Duration::from_millis(50));

    c.bench_function("command_burst_10", |b| {
        b.iter(|| {
            // Send 10 commands in rapid succession
            for _ in 0..10 {
                let _ = playback.send_command(PlaybackCommand::Play);
                let _ = playback.send_command(PlaybackCommand::Pause);
            }
        });
    });
}

/// Benchmark queue operations
fn bench_queue_operations(c: &mut Criterion) {
    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).expect("Failed to create playback");

    let tracks = vec![
        create_test_track("1"),
        create_test_track("2"),
        create_test_track("3"),
    ];
    playback
        .send_command(PlaybackCommand::LoadPlaylist(tracks.clone()))
        .expect("Failed to load playlist");

    std::thread::sleep(Duration::from_millis(50));

    let mut group = c.benchmark_group("queue_operations");

    group.bench_function("add_to_queue", |b| {
        b.iter(|| {
            let track = create_test_track("new");
            playback
                .send_command(black_box(PlaybackCommand::AddToQueueEnd(track)))
                .expect("Failed to add to queue")
        });
    });

    group.bench_function("remove_from_queue", |b| {
        b.iter(|| {
            let _ = playback.send_command(black_box(PlaybackCommand::RemoveFromQueue(1)));
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_cold_start_latency,
    bench_command_latency,
    bench_event_polling_latency,
    bench_playlist_loading,
    bench_concurrent_commands,
    bench_queue_operations
);
criterion_main!(benches);
