//! Performance benchmarks for resampling
//!
//! Run with: cargo bench -p soul-audio --bench resampling_benchmark

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use soul_audio::resampling::{Resampler, ResamplerBackend, ResamplingQuality};
use std::f32::consts::PI;

/// Generate a test signal (1kHz sine wave)
fn generate_test_signal(sample_rate: u32, duration_secs: f32, channels: usize) -> Vec<f32> {
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    let frequency = 1000.0;
    let mut samples = Vec::with_capacity(num_samples * channels);

    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let value = (2.0 * PI * frequency * t).sin();
        for _ in 0..channels {
            samples.push(value);
        }
    }

    samples
}

fn bench_quality_presets(c: &mut Criterion) {
    let mut group = c.benchmark_group("resampling_quality");
    let input_rate = 44100;
    let output_rate = 96000;
    let channels = 2;
    let duration = 1.0; // 1 second

    let input = generate_test_signal(input_rate, duration, channels);
    let input_frames = input.len() / channels;
    group.throughput(Throughput::Elements(input_frames as u64));

    for quality in [
        ResamplingQuality::Fast,
        ResamplingQuality::Balanced,
        ResamplingQuality::High,
        ResamplingQuality::Maximum,
    ] {
        group.bench_with_input(
            BenchmarkId::new("44.1k->96k", format!("{:?}", quality)),
            &input,
            |b, input| {
                let mut resampler = Resampler::new(
                    ResamplerBackend::Rubato,
                    input_rate,
                    output_rate,
                    channels,
                    quality,
                )
                .unwrap();

                b.iter(|| {
                    resampler.reset();
                    black_box(resampler.process(black_box(input)).unwrap())
                });
            },
        );
    }

    group.finish();
}

fn bench_sample_rate_conversions(c: &mut Criterion) {
    let mut group = c.benchmark_group("resampling_rates");
    let channels = 2;
    let duration = 1.0;
    let quality = ResamplingQuality::High;

    let test_cases = vec![
        (44100, 48000, "CD to DAT"),
        (44100, 96000, "CD to 96k"),
        (44100, 192000, "CD to 192k"),
        (48000, 96000, "48k to 96k"),
        (96000, 44100, "96k to CD (downsample)"),
    ];

    for (input_rate, output_rate, label) in test_cases {
        let input = generate_test_signal(input_rate, duration, channels);
        let input_frames = input.len() / channels;
        group.throughput(Throughput::Elements(input_frames as u64));

        group.bench_with_input(BenchmarkId::new(label, ""), &input, |b, input| {
            let mut resampler = Resampler::new(
                ResamplerBackend::Rubato,
                input_rate,
                output_rate,
                channels,
                quality,
            )
            .unwrap();

            b.iter(|| {
                resampler.reset();
                black_box(resampler.process(black_box(input)).unwrap())
            });
        });
    }

    group.finish();
}

fn bench_mono_vs_stereo(c: &mut Criterion) {
    let mut group = c.benchmark_group("resampling_channels");
    let input_rate = 44100;
    let output_rate = 96000;
    let duration = 1.0;
    let quality = ResamplingQuality::High;

    for channels in [1, 2, 6, 8] {
        let input = generate_test_signal(input_rate, duration, channels);
        let input_frames = input.len() / channels;
        group.throughput(Throughput::Elements(input_frames as u64));

        group.bench_with_input(
            BenchmarkId::new("channels", channels),
            &input,
            |b, input| {
                let mut resampler = Resampler::new(
                    ResamplerBackend::Rubato,
                    input_rate,
                    output_rate,
                    channels,
                    quality,
                )
                .unwrap();

                b.iter(|| {
                    resampler.reset();
                    black_box(resampler.process(black_box(input)).unwrap())
                });
            },
        );
    }

    group.finish();
}

fn bench_chunk_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("resampling_chunks");
    let input_rate = 44100;
    let output_rate = 96000;
    let channels = 2;
    let quality = ResamplingQuality::High;
    let total_duration = 5.0; // Process 5 seconds worth in chunks

    let full_input = generate_test_signal(input_rate, total_duration, channels);

    for chunk_samples in [512, 1024, 2048, 4096, 8192] {
        let chunk_size = chunk_samples * channels;
        group.throughput(Throughput::Elements(chunk_samples as u64));

        group.bench_with_input(
            BenchmarkId::new("chunk_size", chunk_samples),
            &full_input,
            |b, full_input| {
                let mut resampler = Resampler::new(
                    ResamplerBackend::Rubato,
                    input_rate,
                    output_rate,
                    channels,
                    quality,
                )
                .unwrap();

                b.iter(|| {
                    resampler.reset();
                    for chunk in full_input.chunks(chunk_size) {
                        black_box(resampler.process(black_box(chunk)).unwrap());
                    }
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_quality_presets,
    bench_sample_rate_conversions,
    bench_mono_vs_stereo,
    bench_chunk_sizes,
);

criterion_main!(benches);
