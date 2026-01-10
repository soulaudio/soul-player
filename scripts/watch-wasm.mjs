#!/usr/bin/env node

/**
 * WASM development watcher
 * Watches Rust source files and rebuilds WASM on changes
 *
 * Usage:
 *   node scripts/watch-wasm.mjs
 *
 * Or add to package.json:
 *   "dev:wasm": "node ../../scripts/watch-wasm.mjs"
 */

import { watch } from 'fs';
import { spawn } from 'child_process';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const projectRoot = join(__dirname, '..');
const wasmSourceDir = join(projectRoot, 'libraries', 'soul-playback', 'src');

// Colors
const colors = {
  reset: '\x1b[0m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  cyan: '\x1b[36m',
};

const log = {
  info: (msg) => console.log(`${colors.cyan}[WASM Watch]${colors.reset} ${msg}`),
  success: (msg) => console.log(`${colors.green}[WASM Watch]${colors.reset} ${msg}`),
  warn: (msg) => console.log(`${colors.yellow}[WASM Watch]${colors.reset} ${msg}`),
};

let building = false;
let buildQueued = false;

/**
 * Build WASM
 */
function buildWasm() {
  if (building) {
    buildQueued = true;
    return;
  }

  building = true;
  buildQueued = false;

  log.info('Change detected, rebuilding...');

  const build = spawn('node', [join(__dirname, 'build-wasm.mjs')], {
    shell: true,
    stdio: 'inherit',
  });

  build.on('close', () => {
    building = false;
    log.success('Build complete. Watching for changes...');

    // If a build was queued during this build, run it now
    if (buildQueued) {
      setTimeout(() => buildWasm(), 100);
    }
  });

  build.on('error', () => {
    building = false;
  });
}

/**
 * Main
 */
function main() {
  log.info(`Watching Rust source files in: ${wasmSourceDir}`);
  log.info('Press Ctrl+C to stop');

  // Initial build
  buildWasm();

  // Watch for changes
  const watcher = watch(wasmSourceDir, { recursive: true }, (eventType, filename) => {
    if (filename && filename.endsWith('.rs')) {
      log.info(`File changed: ${filename}`);
      buildWasm();
    }
  });

  // Graceful shutdown
  process.on('SIGINT', () => {
    log.info('Stopping watcher...');
    watcher.close();
    process.exit(0);
  });
}

main();
