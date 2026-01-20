#!/usr/bin/env node
/**
 * Cross-platform version bumping script for Soul Player
 *
 * Usage: node scripts/bump-version.mjs <version>
 * Example: node scripts/bump-version.mjs 0.1.3
 *
 * This script updates version numbers in:
 * - Workspace Cargo.toml
 * - All package.json files (root + applications)
 * - Tauri tauri.conf.json
 * - .github/release-config.json (for latest.json generation)
 * - Commits, tags, and pushes to origin
 */

import { readFileSync, writeFileSync, readdirSync, statSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { execSync } from 'child_process';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const PROJECT_ROOT = join(__dirname, '..');

// ANSI color codes
const colors = {
  red: '\x1b[31m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  blue: '\x1b[34m',
  reset: '\x1b[0m'
};

function printError(msg) {
  console.error(`${colors.red}❌ ${msg}${colors.reset}`);
}

function printSuccess(msg) {
  console.log(`${colors.green}✓ ${msg}${colors.reset}`);
}

function printWarning(msg) {
  console.log(`${colors.yellow}⚠️  ${msg}${colors.reset}`);
}

function printInfo(msg) {
  console.log(`${colors.blue}ℹ️  ${msg}${colors.reset}`);
}

// Validate semver format
function validateVersion(version) {
  const semverPattern = /^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$/;
  if (!semverPattern.test(version)) {
    printError(`Invalid version format: ${version}`);
    printInfo('Expected format: X.Y.Z (e.g., 0.1.0)');
    printInfo('Or with pre-release: X.Y.Z-alpha.1, X.Y.Z-beta.1, X.Y.Z-rc.1');
    return false;
  }
  return true;
}

// Get current version from workspace Cargo.toml
function getCurrentVersion() {
  try {
    const cargoToml = readFileSync(join(PROJECT_ROOT, 'Cargo.toml'), 'utf8');
    const match = cargoToml.match(/^version = "(.+)"$/m);
    return match ? match[1] : 'unknown';
  } catch (err) {
    return 'unknown';
  }
}

// Update workspace Cargo.toml
function updateCargoToml(newVersion) {
  const filePath = join(PROJECT_ROOT, 'Cargo.toml');
  try {
    let content = readFileSync(filePath, 'utf8');
    content = content.replace(/^version = ".+"$/m, `version = "${newVersion}"`);
    writeFileSync(filePath, content, 'utf8');
    printSuccess(`Updated: Cargo.toml`);
    return true;
  } catch (err) {
    printError(`Failed to update Cargo.toml: ${err.message}`);
    return false;
  }
}

// Update package.json files
function updatePackageJson(filePath, newVersion) {
  try {
    const content = JSON.parse(readFileSync(filePath, 'utf8'));
    content.version = newVersion;
    writeFileSync(filePath, JSON.stringify(content, null, 2) + '\n', 'utf8');
    printSuccess(`Updated: ${filePath.replace(PROJECT_ROOT, '.')}`);
    return true;
  } catch (err) {
    printError(`Failed to update ${filePath}: ${err.message}`);
    return false;
  }
}

// Update tauri.conf.json
function updateTauriConf(newVersion) {
  const filePath = join(PROJECT_ROOT, 'applications/desktop/src-tauri/tauri.conf.json');
  try {
    const content = JSON.parse(readFileSync(filePath, 'utf8'));
    content.version = newVersion;
    writeFileSync(filePath, JSON.stringify(content, null, 2) + '\n', 'utf8');
    printSuccess(`Updated: tauri.conf.json`);
    return true;
  } catch (err) {
    printError(`Failed to update tauri.conf.json: ${err.message}`);
    return false;
  }
}

// Update release-config.json
function updateReleaseConfig(newVersion) {
  const filePath = join(PROJECT_ROOT, '.github/release-config.json');
  try {
    const content = JSON.parse(readFileSync(filePath, 'utf8'));
    content.version = newVersion;
    writeFileSync(filePath, JSON.stringify(content, null, 2) + '\n', 'utf8');
    printSuccess(`Updated: .github/release-config.json`);
    return true;
  } catch (err) {
    printError(`Failed to update .github/release-config.json: ${err.message}`);
    return false;
  }
}

// Find all package.json files in applications directory
function findPackageJsonFiles() {
  const packageJsonFiles = [join(PROJECT_ROOT, 'package.json')];
  const appsDir = join(PROJECT_ROOT, 'applications');

  try {
    const apps = readdirSync(appsDir);
    for (const app of apps) {
      const packageJsonPath = join(appsDir, app, 'package.json');
      try {
        if (statSync(packageJsonPath).isFile()) {
          packageJsonFiles.push(packageJsonPath);
        }
      } catch (err) {
        // File doesn't exist, skip
      }
    }
  } catch (err) {
    printWarning(`Could not read applications directory: ${err.message}`);
  }

  return packageJsonFiles;
}

// Validate critical files after update
function validateUpdates(newVersion) {
  let allValid = true;

  // Validate tauri.conf.json
  const tauriConfPath = join(PROJECT_ROOT, 'applications/desktop/src-tauri/tauri.conf.json');
  try {
    const tauriConf = JSON.parse(readFileSync(tauriConfPath, 'utf8'));
    if (tauriConf.version !== newVersion) {
      printError(`VALIDATION FAILED: tauri.conf.json version mismatch!`);
      printError(`  Expected: ${newVersion}`);
      printError(`  Actual:   ${tauriConf.version}`);
      printWarning('This will cause UI to show wrong version!');
      allValid = false;
    } else {
      printSuccess(`Validation: tauri.conf.json version = ${tauriConf.version} ✓`);
    }
  } catch (err) {
    printError(`Failed to validate tauri.conf.json: ${err.message}`);
    allValid = false;
  }

  // Validate Cargo.toml
  const cargoTomlPath = join(PROJECT_ROOT, 'Cargo.toml');
  try {
    const cargoToml = readFileSync(cargoTomlPath, 'utf8');
    const match = cargoToml.match(/^version = "(.+)"$/m);
    const cargoVersion = match ? match[1] : null;
    if (cargoVersion !== newVersion) {
      printError(`VALIDATION FAILED: Cargo.toml version mismatch!`);
      printError(`  Expected: ${newVersion}`);
      printError(`  Actual:   ${cargoVersion}`);
      allValid = false;
    } else {
      printSuccess(`Validation: Cargo.toml version = ${cargoVersion} ✓`);
    }
  } catch (err) {
    printError(`Failed to validate Cargo.toml: ${err.message}`);
    allValid = false;
  }

  // Validate release-config.json
  const releaseConfigPath = join(PROJECT_ROOT, '.github/release-config.json');
  try {
    const releaseConfig = JSON.parse(readFileSync(releaseConfigPath, 'utf8'));
    if (releaseConfig.version !== newVersion) {
      printError(`VALIDATION FAILED: release-config.json version mismatch!`);
      printError(`  Expected: ${newVersion}`);
      printError(`  Actual:   ${releaseConfig.version}`);
      printWarning('This will cause wrong version in latest.json for auto-updates!');
      allValid = false;
    } else {
      printSuccess(`Validation: release-config.json version = ${releaseConfig.version} ✓`);
    }
  } catch (err) {
    printError(`Failed to validate release-config.json: ${err.message}`);
    allValid = false;
  }

  return allValid;
}

// Run git command
function runGitCommand(command, description) {
  try {
    printInfo(description);
    execSync(command, { cwd: PROJECT_ROOT, stdio: 'inherit' });
    return true;
  } catch (err) {
    printError(`Git command failed: ${command}`);
    return false;
  }
}

// Main function
async function main() {
  console.log('');
  console.log('═══════════════════════════════════════════════════════');
  console.log('  Soul Player Version Bumping Script');
  console.log('═══════════════════════════════════════════════════════');
  console.log('');

  // Check arguments
  if (process.argv.length !== 3) {
    printError('Usage: node scripts/bump-version.mjs <version>');
    console.log('');
    console.log('Examples:');
    console.log('  node scripts/bump-version.mjs 0.1.0');
    console.log('  node scripts/bump-version.mjs 0.2.0-beta.1');
    console.log('  node scripts/bump-version.mjs 1.0.0');
    process.exit(1);
  }

  const newVersion = process.argv[2];

  // Validate version format
  if (!validateVersion(newVersion)) {
    process.exit(1);
  }

  // Get current version
  const currentVersion = getCurrentVersion();

  printInfo(`Current version: ${currentVersion}`);
  printInfo(`New version:     ${newVersion}`);
  console.log('');
  printInfo('Updating version numbers...');
  console.log('');

  let filesUpdated = 0;
  let filesFailed = 0;

  // Update workspace Cargo.toml
  if (updateCargoToml(newVersion)) {
    filesUpdated++;
  } else {
    filesFailed++;
  }

  console.log('');
  printInfo('Updating package.json files...');
  console.log('');

  // Update all package.json files
  const packageJsonFiles = findPackageJsonFiles();
  for (const file of packageJsonFiles) {
    if (updatePackageJson(file, newVersion)) {
      filesUpdated++;
    } else {
      filesFailed++;
    }
  }

  console.log('');

  // Update Tauri config
  if (updateTauriConf(newVersion)) {
    filesUpdated++;
  } else {
    filesFailed++;
  }

  // Update release config
  if (updateReleaseConfig(newVersion)) {
    filesUpdated++;
  } else {
    filesFailed++;
  }

  console.log('');
  printInfo('Validating version updates...');
  console.log('');

  // Validate updates
  const validationPassed = validateUpdates(newVersion);

  console.log('');
  console.log('═══════════════════════════════════════════════════════');

  if (filesFailed === 0 && validationPassed) {
    printSuccess('Version bump complete!');
    printInfo(`Updated ${filesUpdated} file(s)`);
  } else {
    printError('Version bump failed - cannot proceed with commit');
    if (filesFailed > 0) {
      printWarning(`Failed to update ${filesFailed} file(s)`);
    }
    if (!validationPassed) {
      printWarning('Validation failed');
    }
    process.exit(1);
  }

  console.log('');
  printInfo('Committing changes and creating tag...');
  console.log('');

  // Stage all changes
  if (!runGitCommand('git add -A', 'Staging all changes...')) {
    process.exit(1);
  }

  console.log('');

  // Show what will be committed
  console.log('=== Changes to commit ===');
  runGitCommand('git status --short', '');
  console.log('');

  // Commit with conventional commit message
  const commitMessage = `chore: bump version to v${newVersion}

- Updated all Cargo.toml files to v${newVersion}
- Updated all package.json files to v${newVersion}
- Updated tauri.conf.json to v${newVersion}
- Updated .github/release-config.json to v${newVersion}
- Includes previous fixes and improvements`;

  if (!runGitCommand(`git commit -m "${commitMessage}"`, 'Creating commit...')) {
    process.exit(1);
  }
  printSuccess('Commit created successfully');

  console.log('');

  // Create and push tag
  const tagName = `v${newVersion}`;
  if (!runGitCommand(`git tag -a "${tagName}" -m "Release ${newVersion}"`, `Creating tag: ${tagName}`)) {
    process.exit(1);
  }
  printSuccess(`Tag created: ${tagName}`);

  console.log('');
  printInfo('Pushing to origin...');

  // Push commits and tags
  if (!runGitCommand('git push origin main', 'Pushing commits...')) {
    printError('Failed to push commits to origin');
    printWarning('You may need to push manually:');
    console.log('  git push origin main');
    console.log(`  git push origin ${tagName}`);
    process.exit(1);
  }

  if (!runGitCommand(`git push origin "${tagName}"`, 'Pushing tags...')) {
    printError('Failed to push tag to origin');
    printWarning('You may need to push tag manually:');
    console.log(`  git push origin ${tagName}`);
    process.exit(1);
  }

  printSuccess('Successfully pushed commits and tag!');

  console.log('');
  console.log('═══════════════════════════════════════════════════════');
  printSuccess(`Release v${newVersion} initiated!`);
  console.log('');
  printInfo('GitHub Actions will now:');
  console.log(`  • Detect the new tag v${newVersion}`);
  console.log('  • Trigger the release workflow');
  console.log('  • Build installers for Windows, macOS, Linux');
  console.log('  • Build Flatpak package');
  console.log('  • Publish to AUR');
  console.log('  • Create GitHub release');
  console.log('');
  printInfo('Monitor release progress at:');
  console.log('  https://github.com/soulaudio/soul-player/actions');
  console.log('');
  printSuccess('Script complete!');
}

// Run main
main().catch(err => {
  printError(`Unexpected error: ${err.message}`);
  console.error(err);
  process.exit(1);
});
