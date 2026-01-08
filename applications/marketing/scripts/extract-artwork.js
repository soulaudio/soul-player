import { execSync } from 'child_process';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Directories
const audioDir = path.join(__dirname, '../public/demo-audio');
const outputDir = path.join(__dirname, '../public/demo-artwork');

// Create output directory if it doesn't exist
if (!fs.existsSync(outputDir)) {
  fs.mkdirSync(outputDir, { recursive: true });
}

// Files to extract artwork from
const files = [
  { input: 'dark.mp3', output: 'dark.jpg' },
  { input: 'eyes.mp3', output: 'eyes.jpg' }
];

console.log('=== Soul Player: Extracting Demo Artwork ===\n');

files.forEach(({ input, output }) => {
  const inputPath = path.join(audioDir, input);
  const outputPath = path.join(outputDir, output);

  // Check if input file exists
  if (!fs.existsSync(inputPath)) {
    console.warn(`⚠ Input file not found: ${inputPath}`);
    return;
  }

  try {
    // Extract artwork using ffmpeg
    // -i: input file
    // -an: no audio
    // -vcodec copy: copy video stream (image) without re-encoding
    // -y: overwrite output file
    execSync(
      `ffmpeg -i "${inputPath}" -an -vcodec copy "${outputPath}" -y`,
      { stdio: 'pipe' } // Suppress ffmpeg output
    );
    console.log(`✓ Extracted artwork: ${input} → ${output}`);
  } catch (err) {
    // ffmpeg returns non-zero if no artwork found
    console.warn(`✗ No artwork found in: ${input}`);
  }
});

console.log('\n=== Artwork extraction complete ===');
