#!/usr/bin/env node

/**
 * Replace console.log/warn/error with debug.log/warn/error
 * across all TypeScript files in applications/shared/src
 */

import { readFileSync, writeFileSync, readdirSync, statSync } from 'fs';
import { join, relative, dirname } from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const SHARED_SRC = join(__dirname, '..', 'applications', 'shared', 'src');

// Recursive directory traversal
function getAllFiles(dirPath, arrayOfFiles = []) {
  const files = readdirSync(dirPath);

  files.forEach((file) => {
    const filePath = join(dirPath, file);
    if (statSync(filePath).isDirectory()) {
      arrayOfFiles = getAllFiles(filePath, arrayOfFiles);
    } else if (file.match(/\.(ts|tsx)$/) && !file.match(/\.test\.(ts|tsx)$/)) {
      arrayOfFiles.push(filePath);
    }
  });

  return arrayOfFiles;
}

// Calculate import path based on file location
function getImportPath(filePath) {
  const rel = relative(SHARED_SRC, filePath);
  const depth = rel.split(/[/\\]/).length - 1;

  return '../'.repeat(depth) + 'utils/debug';
}

// Process a single file
function processFile(filePath) {
  let content = readFileSync(filePath, 'utf8');
  let modified = false;

  // Check if file contains console statements
  if (!/console\.(log|warn|error)/.test(content)) {
    return false;
  }

  console.log(`Processing: ${relative(process.cwd(), filePath)}`);

  // Check if debug is already imported
  if (!/import\s*{\s*debug\s*}\s*from/.test(content)) {
    const importPath = getImportPath(filePath);

    // Find last import statement
    const lines = content.split('\n');
    let lastImportIndex = -1;

    for (let i = 0; i < lines.length; i++) {
      if (/^import\s/.test(lines[i].trim())) {
        lastImportIndex = i;
      }
    }

    if (lastImportIndex !== -1) {
      // Insert debug import after last import
      lines.splice(lastImportIndex + 1, 0, `import { debug } from '${importPath}';`);
      content = lines.join('\n');
      modified = true;
      console.log('  ✓ Added debug import');
    }
  }

  // Replace console statements
  const originalContent = content;

  content = content.replace(/console\.log\(/g, 'debug.log(');
  content = content.replace(/console\.warn\(/g, 'debug.warn(');
  content = content.replace(/console\.error\(/g, 'debug.error(');

  if (content !== originalContent) {
    modified = true;
    console.log('  ✓ Replaced console statements');
  }

  if (modified) {
    writeFileSync(filePath, content, 'utf8');
    return true;
  }

  return false;
}

// Main execution
console.log('Starting console.log replacement...\n');

const files = getAllFiles(SHARED_SRC);
console.log(`Found ${files.length} TypeScript files\n`);

let modifiedCount = 0;

files.forEach((file) => {
  if (processFile(file)) {
    modifiedCount++;
  }
});

console.log(`\n✓ Complete! Modified ${modifiedCount} files`);
