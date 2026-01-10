#!/usr/bin/env node

/**
 * Cross-platform WASM build script
 * Builds soul-playback WASM module for the marketing demo
 * Runs automatically before dev/build via npm lifecycle hooks
 */

import { spawn } from 'child_process';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import { existsSync } from 'fs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Paths
const projectRoot = join(__dirname, '..');
const wasmSourceDir = join(projectRoot, 'libraries', 'soul-playback');
const wasmOutputDir = join(projectRoot, 'applications', 'marketing', 'src', 'wasm', 'soul-playback');

// Colors for terminal output
const colors = {
  reset: '\x1b[0m',
  bright: '\x1b[1m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  red: '\x1b[31m',
  cyan: '\x1b[36m',
};

const log = {
  info: (msg) => console.log(`${colors.cyan}[WASM]${colors.reset} ${msg}`),
  success: (msg) => console.log(`${colors.green}[WASM]${colors.reset} ${msg}`),
  warn: (msg) => console.log(`${colors.yellow}[WASM]${colors.reset} ${msg}`),
  error: (msg) => console.error(`${colors.red}[WASM]${colors.reset} ${msg}`),
};

/**
 * Check if wasm-pack is installed
 */
function checkWasmPack() {
  return new Promise((resolve) => {
    const check = spawn('wasm-pack', ['--version'], { shell: true });
    check.on('close', (code) => {
      resolve(code === 0);
    });
    check.on('error', () => {
      resolve(false);
    });
  });
}

/**
 * Build WASM module
 */
function buildWasm() {
  return new Promise((resolve, reject) => {
    log.info('Building soul-playback WASM module...');
    log.info(`Source: ${wasmSourceDir}`);
    log.info(`Output: ${wasmOutputDir}`);

    const args = [
      'build',
      '--target', 'web',
      '--out-dir', wasmOutputDir,
      '--release',
      '--',
      '--features', 'wasm',
    ];

    const build = spawn('wasm-pack', args, {
      cwd: wasmSourceDir,
      shell: true,
      stdio: 'inherit',
    });

    build.on('close', (code) => {
      if (code === 0) {
        log.success('WASM build complete!');
        log.success(`Output: ${wasmOutputDir}`);
        resolve();
      } else {
        reject(new Error(`wasm-pack exited with code ${code}`));
      }
    });

    build.on('error', (err) => {
      reject(err);
    });
  });
}

/**
 * Main execution
 */
async function main() {
  try {
    // Check if source directory exists
    if (!existsSync(wasmSourceDir)) {
      log.error(`Source directory not found: ${wasmSourceDir}`);
      process.exit(1);
    }

    // Check if wasm-pack is installed
    log.info('Checking for wasm-pack...');
    const hasWasmPack = await checkWasmPack();

    if (!hasWasmPack) {
      log.error('wasm-pack is not installed!');
      log.info('Install with: cargo install wasm-pack');
      log.info('Or visit: https://rustwasm.github.io/wasm-pack/installer/');
      process.exit(1);
    }

    log.success('wasm-pack found');

    // Build WASM
    await buildWasm();

    process.exit(0);
  } catch (error) {
    log.error('WASM build failed:');
    log.error(error.message);
    process.exit(1);
  }
}

main();
