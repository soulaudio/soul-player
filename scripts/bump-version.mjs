#!/usr/bin/env node
/**
 * Cross-platform version bumping script for Soul Player
 *
 * Follows best practices for Rust monorepo + Tauri projects:
 * - Pre-flight validation (git status, dependency sync, etc.)
 * - Dry-run mode for safe testing
 * - Backup/rollback capability
 * - Semver validation and comparison
 * - Tauri-specific dependency checks
 * - Conventional commit formatting
 *
 * Usage: node scripts/bump-version.mjs [options] <version>
 *
 * Options:
 *   --dry-run, -d    Preview changes without making them
 *   --skip-git, -s   Skip git operations (commit, tag, push)
 *   --help, -h       Show help
 *
 * Examples:
 *   node scripts/bump-version.mjs 0.1.5
 *   node scripts/bump-version.mjs --dry-run 0.2.0
 *   node scripts/bump-version.mjs --skip-git 1.0.0-beta.1
 */

import { readFileSync, writeFileSync, readdirSync, statSync, copyFileSync, unlinkSync } from 'fs';
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
  cyan: '\x1b[36m',
  reset: '\x1b[0m'
};

// Configuration
let DRY_RUN = false;
let SKIP_GIT = false;
const BACKUP_FILES = new Map(); // For rollback

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

function printDryRun(msg) {
  console.log(`${colors.cyan}[DRY-RUN] ${msg}${colors.reset}`);
}

// ============================================================================
// Semver Utilities
// ============================================================================

function parseVersion(version) {
  const match = version.match(/^(\d+)\.(\d+)\.(\d+)(?:-(.+))?$/);
  if (!match) return null;
  return {
    major: parseInt(match[1]),
    minor: parseInt(match[2]),
    patch: parseInt(match[3]),
    prerelease: match[4] || null,
    toString() { return this.prerelease ? `${this.major}.${this.minor}.${this.patch}-${this.prerelease}` : `${this.major}.${this.minor}.${this.patch}`; }
  };
}

function compareVersions(v1, v2) {
  const parsed1 = parseVersion(v1);
  const parsed2 = parseVersion(v2);

  if (!parsed1 || !parsed2) return 0;

  if (parsed1.major !== parsed2.major) return parsed1.major - parsed2.major;
  if (parsed1.minor !== parsed2.minor) return parsed1.minor - parsed2.minor;
  if (parsed1.patch !== parsed2.patch) return parsed1.patch - parsed2.patch;

  // Handle prerelease: 1.0.0-beta < 1.0.0
  if (parsed1.prerelease && !parsed2.prerelease) return -1;
  if (!parsed1.prerelease && parsed2.prerelease) return 1;
  if (parsed1.prerelease && parsed2.prerelease) {
    return parsed1.prerelease.localeCompare(parsed2.prerelease);
  }

  return 0;
}

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

// ============================================================================
// Git Utilities
// ============================================================================

function runCommand(command, description, options = {}) {
  const { silent = false, ignoreError = false } = options;
  try {
    if (!silent && description) {
      printInfo(description);
    }
    const result = execSync(command, {
      cwd: PROJECT_ROOT,
      encoding: 'utf8',
      stdio: silent ? 'pipe' : 'inherit'
    });
    return { success: true, output: result };
  } catch (err) {
    if (ignoreError) {
      return { success: false, output: err.stdout || '', error: err.message };
    }
    printError(`Command failed: ${command}`);
    throw err;
  }
}

function checkGitStatus() {
  const result = runCommand('git status --porcelain', '', { silent: true });
  if (result.output.trim()) {
    printError('Git working directory is not clean');
    console.log('');
    console.log('Uncommitted changes:');
    console.log(result.output);
    console.log('');
    printInfo('Please commit or stash your changes before bumping version');
    return false;
  }
  printSuccess('Git working directory is clean');
  return true;
}

function checkGitBranch() {
  const result = runCommand('git rev-parse --abbrev-ref HEAD', '', { silent: true });
  const branch = result.output.trim();

  if (branch !== 'main' && branch !== 'master') {
    printWarning(`You are on branch '${branch}', not 'main' or 'master'`);
    printInfo('Version bumps are typically done on the main branch');
    // Don't fail, just warn
  } else {
    printSuccess(`On branch '${branch}'`);
  }
  return true;
}

// ============================================================================
// File Backup and Rollback
// ============================================================================

function backupFile(filePath) {
  if (DRY_RUN) return;

  const backupPath = `${filePath}.backup`;
  try {
    copyFileSync(filePath, backupPath);
    BACKUP_FILES.set(filePath, backupPath);
  } catch (err) {
    printWarning(`Failed to backup ${filePath}: ${err.message}`);
  }
}

function rollbackChanges() {
  if (BACKUP_FILES.size === 0) return;

  printWarning('Rolling back changes...');
  for (const [original, backup] of BACKUP_FILES) {
    try {
      copyFileSync(backup, original);
      unlinkSync(backup);
      printSuccess(`Restored: ${original}`);
    } catch (err) {
      printError(`Failed to restore ${original}: ${err.message}`);
    }
  }
  BACKUP_FILES.clear();
}

function cleanupBackups() {
  for (const [, backup] of BACKUP_FILES) {
    try {
      unlinkSync(backup);
    } catch (err) {
      // Ignore cleanup errors
    }
  }
  BACKUP_FILES.clear();
}

// ============================================================================
// Version Reading Functions
// ============================================================================

function getCurrentVersion() {
  try {
    const cargoToml = readFileSync(join(PROJECT_ROOT, 'Cargo.toml'), 'utf8');
    const match = cargoToml.match(/^version = "(.+)"$/m);
    return match ? match[1] : 'unknown';
  } catch (err) {
    return 'unknown';
  }
}

function getTauriVersions() {
  try {
    const cargoToml = readFileSync(join(PROJECT_ROOT, 'applications/desktop/src-tauri/Cargo.toml'), 'utf8');
    const packageJson = JSON.parse(readFileSync(join(PROJECT_ROOT, 'applications/desktop/package.json'), 'utf8'));

    const tauriMatch = cargoToml.match(/tauri\s*=\s*{[^}]*version\s*=\s*"([^"]+)"/);
    const tauriBuildMatch = cargoToml.match(/tauri-build\s*=\s*{[^}]*version\s*=\s*"([^"]+)"/);

    return {
      tauri: tauriMatch ? tauriMatch[1] : 'unknown',
      tauriBuild: tauriBuildMatch ? tauriBuildMatch[1] : 'unknown',
      tauriApi: packageJson.dependencies?.['@tauri-apps/api'] || 'unknown'
    };
  } catch (err) {
    return { tauri: 'unknown', tauriBuild: 'unknown', tauriApi: 'unknown' };
  }
}

// ============================================================================
// Pre-flight Checks
// ============================================================================

function checkTauriDependencies() {
  const versions = getTauriVersions();

  // Extract minor versions (e.g., "2.9.0" -> "2.9")
  const getMinor = (v) => {
    if (v === 'unknown' || v.startsWith('^') || v.startsWith('~')) {
      return v.replace(/^[~^]/, '').split('.').slice(0, 2).join('.');
    }
    return v.split('.').slice(0, 2).join('.');
  };

  const tauriMinor = getMinor(versions.tauri);
  const tauriBuildMinor = getMinor(versions.tauriBuild);
  const tauriApiMinor = getMinor(versions.tauriApi);

  console.log('');
  printInfo('Tauri dependency versions:');
  console.log(`  tauri:        ${versions.tauri} (minor: ${tauriMinor})`);
  console.log(`  tauri-build:  ${versions.tauriBuild} (minor: ${tauriBuildMinor})`);
  console.log(`  @tauri-apps/api: ${versions.tauriApi} (minor: ${tauriApiMinor})`);
  console.log('');

  // Check if minor versions match
  if (tauriMinor !== tauriBuildMinor) {
    printWarning('tauri and tauri-build have different minor versions');
    printInfo('This may cause compatibility issues - consider syncing them');
  }

  if (tauriMinor !== tauriApiMinor) {
    printWarning('Rust tauri and JS @tauri-apps/api have different minor versions');
    printInfo('Tauri recommends keeping these in sync for compatibility');
  }

  if (tauriMinor === tauriBuildMinor && tauriMinor === tauriApiMinor) {
    printSuccess('Tauri dependencies are in sync');
  }

  return true; // Don't fail, just warn
}

function runPreflightChecks() {
  console.log('');
  console.log('═══════════════════════════════════════════════════════');
  console.log('  Pre-flight Checks');
  console.log('═══════════════════════════════════════════════════════');
  console.log('');

  const checks = [
    { name: 'Git status', fn: checkGitStatus },
    { name: 'Git branch', fn: checkGitBranch },
    { name: 'Tauri dependencies', fn: checkTauriDependencies }
  ];

  for (const check of checks) {
    if (!check.fn()) {
      printError(`Pre-flight check failed: ${check.name}`);
      return false;
    }
  }

  console.log('');
  printSuccess('All pre-flight checks passed');
  return true;
}

// ============================================================================
// File Update Functions
// ============================================================================

function updateCargoToml(newVersion) {
  const filePath = join(PROJECT_ROOT, 'Cargo.toml');
  backupFile(filePath);

  try {
    let content = readFileSync(filePath, 'utf8');
    const oldContent = content;
    content = content.replace(/^version = ".+"$/m, `version = "${newVersion}"`);

    if (DRY_RUN) {
      printDryRun(`Would update: Cargo.toml`);
      return true;
    }

    if (content === oldContent) {
      printWarning('Cargo.toml: no changes detected');
      return true;
    }

    writeFileSync(filePath, content, 'utf8');
    printSuccess(`Updated: Cargo.toml`);
    return true;
  } catch (err) {
    printError(`Failed to update Cargo.toml: ${err.message}`);
    return false;
  }
}

function updatePackageJson(filePath, newVersion) {
  backupFile(filePath);

  try {
    const content = JSON.parse(readFileSync(filePath, 'utf8'));
    const oldVersion = content.version;
    content.version = newVersion;

    if (DRY_RUN) {
      printDryRun(`Would update: ${filePath.replace(PROJECT_ROOT, '.')} (${oldVersion} → ${newVersion})`);
      return true;
    }

    writeFileSync(filePath, JSON.stringify(content, null, 2) + '\n', 'utf8');
    printSuccess(`Updated: ${filePath.replace(PROJECT_ROOT, '.')}`);
    return true;
  } catch (err) {
    printError(`Failed to update ${filePath}: ${err.message}`);
    return false;
  }
}

function updateTauriConf(newVersion) {
  const filePath = join(PROJECT_ROOT, 'applications/desktop/src-tauri/tauri.conf.json');
  backupFile(filePath);

  try {
    const content = JSON.parse(readFileSync(filePath, 'utf8'));
    const oldVersion = content.version;
    content.version = newVersion;

    if (DRY_RUN) {
      printDryRun(`Would update: tauri.conf.json (${oldVersion} → ${newVersion})`);
      return true;
    }

    writeFileSync(filePath, JSON.stringify(content, null, 2) + '\n', 'utf8');
    printSuccess(`Updated: tauri.conf.json`);
    return true;
  } catch (err) {
    printError(`Failed to update tauri.conf.json: ${err.message}`);
    return false;
  }
}

function updateReleaseConfig(newVersion) {
  const filePath = join(PROJECT_ROOT, '.github/release-config.json');
  backupFile(filePath);

  try {
    const content = JSON.parse(readFileSync(filePath, 'utf8'));
    const oldVersion = content.version;
    content.version = newVersion;

    if (DRY_RUN) {
      printDryRun(`Would update: .github/release-config.json (${oldVersion} → ${newVersion})`);
      return true;
    }

    writeFileSync(filePath, JSON.stringify(content, null, 2) + '\n', 'utf8');
    printSuccess(`Updated: .github/release-config.json`);
    return true;
  } catch (err) {
    printError(`Failed to update .github/release-config.json: ${err.message}`);
    return false;
  }
}

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

// ============================================================================
// Validation Functions
// ============================================================================

function validateUpdates(newVersion) {
  console.log('');
  printInfo('Validating version updates...');
  console.log('');

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

// ============================================================================
// Git Operations
// ============================================================================

function createCommitAndTag(newVersion, currentVersion) {
  if (SKIP_GIT) {
    printInfo('Skipping git operations (--skip-git)');
    return true;
  }

  if (DRY_RUN) {
    printDryRun('Would create commit and tag');
    printDryRun(`Commit message: "chore: bump version to v${newVersion}"`);
    printDryRun(`Tag: v${newVersion}`);
    return true;
  }

  console.log('');
  printInfo('Committing changes and creating tag...');
  console.log('');

  // Stage all changes
  if (!runCommand('git add -A', 'Staging all changes...').success) {
    return false;
  }

  console.log('');

  // Show what will be committed
  console.log('═══ Changes to commit ═══');
  runCommand('git status --short', '');
  console.log('');

  // Create conventional commit message
  const commitMessage = `chore: bump version to v${newVersion}

- Updated workspace Cargo.toml to v${newVersion}
- Updated all package.json files to v${newVersion}
- Updated tauri.conf.json to v${newVersion}
- Updated .github/release-config.json to v${newVersion}`;

  // Commit
  const commitCmd = `git commit -m "${commitMessage.replace(/"/g, '\\"')}"`;
  if (!runCommand(commitCmd, 'Creating commit...').success) {
    return false;
  }
  printSuccess('Commit created successfully');

  console.log('');

  // Create and push tag
  const tagName = `v${newVersion}`;
  if (!runCommand(`git tag -a "${tagName}" -m "Release ${newVersion}"`, `Creating tag: ${tagName}`).success) {
    return false;
  }
  printSuccess(`Tag created: ${tagName}`);

  console.log('');
  printInfo('Pushing to origin...');

  // Push commits
  if (!runCommand('git push origin main', 'Pushing commits...').success) {
    printError('Failed to push commits to origin');
    printWarning('You may need to push manually:');
    console.log('  git push origin main');
    console.log(`  git push origin ${tagName}`);
    return false;
  }

  // Push tag
  if (!runCommand(`git push origin "${tagName}"`, 'Pushing tag...').success) {
    printError('Failed to push tag to origin');
    printWarning('You may need to push tag manually:');
    console.log(`  git push origin ${tagName}`);
    return false;
  }

  printSuccess('Successfully pushed commits and tag!');
  return true;
}

// ============================================================================
// Main Function
// ============================================================================

async function main() {
  // Parse command-line arguments
  const args = process.argv.slice(2);
  let newVersion = null;

  for (const arg of args) {
    if (arg === '--dry-run' || arg === '-d') {
      DRY_RUN = true;
    } else if (arg === '--skip-git' || arg === '-s') {
      SKIP_GIT = true;
    } else if (arg === '--help' || arg === '-h') {
      console.log('Usage: node scripts/bump-version.mjs [options] <version>');
      console.log('');
      console.log('Options:');
      console.log('  --dry-run, -d    Preview changes without making them');
      console.log('  --skip-git, -s   Skip git operations (commit, tag, push)');
      console.log('  --help, -h       Show this help message');
      console.log('');
      console.log('Examples:');
      console.log('  node scripts/bump-version.mjs 0.1.5');
      console.log('  node scripts/bump-version.mjs --dry-run 0.2.0');
      console.log('  node scripts/bump-version.mjs --skip-git 1.0.0-beta.1');
      process.exit(0);
    } else if (!arg.startsWith('-')) {
      newVersion = arg;
    }
  }

  console.log('');
  console.log('═══════════════════════════════════════════════════════');
  console.log('  Soul Player Version Bumping Script');
  if (DRY_RUN) {
    console.log('  [DRY-RUN MODE - No changes will be made]');
  }
  console.log('═══════════════════════════════════════════════════════');
  console.log('');

  // Check if version was provided
  if (!newVersion) {
    printError('Usage: node scripts/bump-version.mjs [options] <version>');
    console.log('');
    console.log('Examples:');
    console.log('  node scripts/bump-version.mjs 0.1.5');
    console.log('  node scripts/bump-version.mjs --dry-run 0.2.0');
    console.log('  node scripts/bump-version.mjs --skip-git 1.0.0-beta.1');
    console.log('');
    printInfo('Use --help for more information');
    process.exit(1);
  }

  // Validate version format
  if (!validateVersion(newVersion)) {
    process.exit(1);
  }

  // Get current version
  const currentVersion = getCurrentVersion();

  printInfo(`Current version: ${currentVersion}`);
  printInfo(`New version:     ${newVersion}`);

  // Compare versions
  const comparison = compareVersions(newVersion, currentVersion);
  if (comparison < 0) {
    printWarning(`New version ${newVersion} is LOWER than current ${currentVersion}`);
    printInfo('This is a version downgrade - are you sure?');
  } else if (comparison === 0) {
    printError(`New version ${newVersion} is the SAME as current ${currentVersion}`);
    printInfo('Version must be different from current version');
    process.exit(1);
  } else {
    printSuccess(`Version will be bumped from ${currentVersion} → ${newVersion}`);
  }

  // Run pre-flight checks (skip if dry-run)
  if (!DRY_RUN && !runPreflightChecks()) {
    process.exit(1);
  }

  console.log('');
  printInfo('Updating version numbers...');
  console.log('');

  let filesUpdated = 0;
  let filesFailed = 0;

  try {
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

    // Validate updates (skip if dry-run)
    let validationPassed = true;
    if (!DRY_RUN) {
      validationPassed = validateUpdates(newVersion);
    }

    console.log('');
    console.log('═══════════════════════════════════════════════════════');

    if (filesFailed === 0 && validationPassed) {
      if (DRY_RUN) {
        printSuccess('Dry-run complete - no changes were made');
        printInfo(`Would update ${filesUpdated} file(s)`);
      } else {
        printSuccess('Version bump complete!');
        printInfo(`Updated ${filesUpdated} file(s)`);
      }
    } else {
      printError('Version bump failed');
      if (filesFailed > 0) {
        printWarning(`Failed to update ${filesFailed} file(s)`);
      }
      if (!validationPassed) {
        printWarning('Validation failed');
      }

      if (!DRY_RUN) {
        rollbackChanges();
      }
      process.exit(1);
    }

    // Git operations
    if (!DRY_RUN && !SKIP_GIT) {
      console.log('');
      if (!createCommitAndTag(newVersion, currentVersion)) {
        rollbackChanges();
        process.exit(1);
      }

      console.log('');
      console.log('═══════════════════════════════════════════════════════');
      printSuccess(`Release v${newVersion} initiated!`);
      console.log('');
      printInfo('GitHub Actions will now:');
      console.log(`  • Detect the new tag v${newVersion}`);
      console.log('  • Trigger the release workflow');
      console.log('  • Build installers for Windows, macOS, Linux');
      console.log('  • Create GitHub release with auto-generated changelog');
      console.log('  • Generate latest.json for auto-updater');
      console.log('');
      printInfo('Monitor release progress at:');
      console.log('  https://github.com/soulaudio/soul-player/actions');
    }

    console.log('');
    printSuccess('Script complete!');

    // Cleanup backups on success
    if (!DRY_RUN) {
      cleanupBackups();
    }

  } catch (err) {
    printError(`Unexpected error: ${err.message}`);
    console.error(err);

    if (!DRY_RUN) {
      rollbackChanges();
    }
    process.exit(1);
  }
}

// Run main with error handling
main().catch(err => {
  printError(`Unexpected error: ${err.message}`);
  console.error(err);
  rollbackChanges();
  process.exit(1);
});
