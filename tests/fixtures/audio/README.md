# Test Audio Fixtures

This directory contains audio files used for E2E testing.

## Structure

```
audio/
├── README.md           # This file
├── generated/          # Auto-generated test files (gitignored)
└── reference/          # Reference audio files (committed)
```

## Generating Test Audio

Use the generation script to create test audio files:

```bash
# From repository root
bash scripts/generate-test-audio.sh
```

This creates:
- Various format files (WAV, MP3, FLAC)
- Different durations (short, medium, long)
- Different sample rates (44.1kHz, 48kHz, 96kHz)
- Stereo and mono files
- Files with metadata tags

## Reference Files

Small reference files can be committed to git for quick testing.
Keep files under 100KB.

## Generated Files

Generated test files are gitignored. Run the generation script
before running E2E tests if needed.
