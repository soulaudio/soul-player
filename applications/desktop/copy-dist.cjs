#!/usr/bin/env node
/**
 * Cross-platform script to copy dist folder to src-tauri/dist
 * Used by Tauri's beforeBuildCommand to ensure frontend assets are in the correct location
 */

const fs = require('fs');
const path = require('path');

const srcDir = path.join(__dirname, 'dist');
const destDir = path.join(__dirname, 'src-tauri', 'dist');

console.log('[copy-dist] Copying frontend assets...');
console.log(`[copy-dist]   FROM: ${srcDir}`);
console.log(`[copy-dist]   TO: ${destDir}`);

// Remove existing dest if it exists
if (fs.existsSync(destDir)) {
  console.log('[copy-dist] Removing existing destination...');
  fs.rmSync(destDir, { recursive: true, force: true });
}

// Copy recursively
function copyRecursive(src, dest) {
  const stats = fs.statSync(src);

  if (stats.isDirectory()) {
    fs.mkdirSync(dest, { recursive: true });
    const entries = fs.readdirSync(src);

    for (const entry of entries) {
      copyRecursive(
        path.join(src, entry),
        path.join(dest, entry)
      );
    }
  } else {
    fs.copyFileSync(src, dest);
  }
}

copyRecursive(srcDir, destDir);

// Verify the copy
const files = fs.readdirSync(destDir);
console.log(`[copy-dist] ✓ Copied ${files.length} items to src-tauri/dist/`);
console.log('[copy-dist] Contents:', files.join(', '));

// Verify HTML files specifically
const htmlFiles = files.filter(f => f.endsWith('.html'));
console.log(`[copy-dist] HTML files: ${htmlFiles.join(', ')}`);

if (htmlFiles.length === 0) {
  console.error('[copy-dist] ❌ ERROR: No HTML files found in copied dist!');
  process.exit(1);
}

console.log('[copy-dist] ✓ Copy complete');
