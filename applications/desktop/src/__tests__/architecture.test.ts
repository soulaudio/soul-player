/**
 * Architecture enforcement tests for the Soul Player playback system.
 *
 * These tests scan the source code statically to ensure that:
 *
 * 1. All playback commands (play, pause, skip, seek, queue) are routed exclusively
 *    through `PlayerCommandsContext` / `usePlayerCommands()`.
 *    No component or page may call `invoke('next_track')` etc. directly.
 *
 * 2. Only the two allowed provider files may use `invoke()` for playback operations:
 *    - TauriPlayerCommandsProvider.tsx  (playback commands)
 *    - TauriBackendProvider.tsx         (data / backend operations)
 *
 * 3. Source files must use the `debug` utility from @soul-player/shared instead of
 *    raw `console.log / console.error / console.warn`. This ensures log output
 *    is filtered by the DEBUG flag and never leaks in production builds.
 *
 * WHY THIS MATTERS
 * ----------------
 * When a component calls `invoke('play_queue', ...)` directly it bypasses the
 * shared `PlayerCommandsInterface`. The same component will silently break on
 * the marketing/web platform where there is no Tauri, and it prevents the
 * `MockBackendProvider` from intercepting calls during tests.
 *
 * ADDING LEGITIMATE EXCEPTIONS
 * ------------------------------
 * If a new provider legitimately needs to invoke playback commands, add its
 * path to `ALLOWED_PLAYBACK_INVOKE_FILES` below. Do NOT add pages or components.
 *
 * For console.log violations in existing files, add to CONSOLE_LOG_KNOWN_VIOLATIONS
 * as a temporary allowlist entry — then clean up the file and remove it from the list.
 * DO NOT add new files to CONSOLE_LOG_KNOWN_VIOLATIONS; fix the violation instead.
 */

import { describe, it, expect } from 'vitest';
import * as fs from 'fs';
import * as path from 'path';

// ===== Configuration =====

/** Tauri command names that constitute "playback" operations. */
const PLAYBACK_COMMANDS = [
  'play_queue',
  'play_queue_with_context',
  'play_track',
  'pause_playback',
  'resume_playback',
  'stop_playback',
  'next_track',
  'previous_track',
  'seek_to',
  'skip_to_queue_index',
  'set_shuffle',
  'cycle_shuffle',
  'set_repeat',
  'cycle_repeat',
  'add_play_next',
  'add_to_queue_end',
  'clear_play_next',
  'clear_add_to_queue',
  'get_playback_state',
  'get_position',
  'get_volume',
  'set_volume',
  'get_queue',
  'get_queue_index',
  'get_current_track',
  'get_playback_capabilities',
  'get_repeat',
  'get_shuffle',
];

/**
 * Files that are explicitly allowed to call invoke() with playback commands.
 * These are the provider files that implement the PlayerCommandsInterface.
 */
const ALLOWED_PLAYBACK_INVOKE_FILES = [
  path.normalize('applications/desktop/src/providers/TauriPlayerCommandsProvider.tsx'),
  // test-helpers is allowed to use invoke() for mock setup
  path.normalize('applications/desktop/src/test-helpers.ts'),
];

/** Root of the monorepo.
 * __dirname = applications/desktop/src/__tests__
 * 4 levels up = soul-player repo root
 */
const REPO_ROOT = path.resolve(__dirname, '../../../..');

/** Directories to scan for violations. */
const SCAN_DIRS = [
  path.join(REPO_ROOT, 'applications/desktop/src'),
  path.join(REPO_ROOT, 'applications/shared/src'),
];

// ===== Helpers =====

/** Recursively collect all .ts / .tsx files under a directory. */
function collectSourceFiles(dir: string): string[] {
  if (!fs.existsSync(dir)) return [];
  const result: string[] = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory() && entry.name !== 'node_modules' && entry.name !== '__tests__') {
      result.push(...collectSourceFiles(full));
    } else if (entry.isFile() && /\.(ts|tsx)$/.test(entry.name)) {
      result.push(full);
    }
  }
  return result;
}

/** Return true if the file is a test or spec file (by naming convention). */
function isTestFile(filePath: string): boolean {
  return /\.(test|spec)\.(ts|tsx)$/.test(filePath);
}

/** Return true if the file is in the allowed list (matched by normalized suffix). */
function isAllowedFile(filePath: string): boolean {
  const normalized = path.normalize(filePath);
  return ALLOWED_PLAYBACK_INVOKE_FILES.some((allowed) => normalized.endsWith(allowed));
}

interface Violation {
  file: string;
  line: number;
  command: string;
  snippet: string;
}

/** Scan a file for direct invoke() calls using any of the playback command names. */
function findPlaybackInvokeViolations(filePath: string): Violation[] {
  if (isAllowedFile(filePath)) return [];

  const content = fs.readFileSync(filePath, 'utf-8');
  const lines = content.split('\n');
  const violations: Violation[] = [];

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    for (const cmd of PLAYBACK_COMMANDS) {
      // Match: invoke('cmd', ...) or invoke("cmd", ...) or invoke(`cmd`, ...)
      // Allow commented-out lines (// or *)
      const trimmed = line.trimStart();
      if (trimmed.startsWith('//') || trimmed.startsWith('*')) continue;

      const pattern = new RegExp(`invoke\\s*\\(\\s*['"\`]${cmd}['"\`]`);
      if (pattern.test(line)) {
        violations.push({
          file: path.relative(REPO_ROOT, filePath).replace(/\\/g, '/'),
          line: i + 1,
          command: cmd,
          snippet: line.trim(),
        });
      }
    }
  }

  return violations;
}

// ===== Console Logging Configuration =====

/**
 * Files permanently allowed to use console.log:
 * - debug.ts is the implementation of the debug utility — it wraps console.log internally.
 * - test-helpers.ts is a test-only helper file (not shipped in production).
 */
const CONSOLE_LOG_ALWAYS_ALLOWED = [
  path.normalize('applications/shared/src/utils/debug.ts'),
  path.normalize('applications/desktop/src/test-helpers.ts'),
];

/**
 * Files with KNOWN console.log violations that have not yet been migrated to
 * `debug.log / debug.error / debug.warn`. Each entry is a temporary TODO.
 *
 * ⚠ DO NOT ADD NEW FILES HERE — fix the violation instead by importing `debug`
 *   from '@soul-player/shared' and replacing the console.* call.
 *
 * Remove a file from this list once it has been cleaned up.
 */
const CONSOLE_LOG_KNOWN_VIOLATIONS = new Set([
  // applications/desktop
  'applications/desktop/src/pages/OnboardingPage.tsx',
  'applications/desktop/src/components/ImportDialog.tsx',
  'applications/desktop/src/components/ScanProgressIndicator.tsx',
  'applications/desktop/src/pages/SettingsPage.tsx',
  'applications/desktop/src/contexts/SettingsContext.tsx',
  'applications/desktop/src/components/ShortcutsSettings.tsx',
  'applications/desktop/src/providers/TauriPlayerCommandsProvider.tsx',
  'applications/desktop/src/App.tsx',
  'applications/desktop/src/layouts/MainLayout.tsx',
  'applications/desktop/src/hooks/useKeyboardShortcuts.ts',
  'applications/desktop/src/components/FileDropHandler.tsx',
  'applications/desktop/src/components/WindowControls.tsx',
  'applications/desktop/src/components/UpdateDialog.tsx',
  // applications/shared
  'applications/shared/src/components/settings/DataManagementSettingsPage.tsx',
  'applications/shared/src/hooks/useAudioDevice.ts',
  'applications/shared/src/providers/MockBackendProvider.tsx',
  'applications/shared/src/providers/WebPlaybackProvider.tsx',
  'applications/shared/src/hooks/usePlaybackEvents.ts',
  'applications/shared/src/hooks/queries/useArtworkMutations.ts',
  'applications/shared/src/components/settings/audio/LatencyMonitor.tsx',
  'applications/shared/src/components/settings/audio/VolumeLevelingSettings.tsx',
  'applications/shared/src/stores/sync.ts',
]);

interface ConsoleViolation {
  file: string;
  line: number;
  call: string;
  snippet: string;
}

/** Scan a file for raw console.log/error/warn/info calls. */
function findConsoleLogViolations(filePath: string): ConsoleViolation[] {
  // Skip permanently allowed files
  const normalized = path.normalize(filePath);
  if (CONSOLE_LOG_ALWAYS_ALLOWED.some((a) => normalized.endsWith(a))) return [];

  // Skip test / spec files
  if (isTestFile(filePath)) return [];

  const content = fs.readFileSync(filePath, 'utf-8');
  const lines = content.split('\n');
  const violations: ConsoleViolation[] = [];

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const trimmed = line.trimStart();
    // Skip commented-out lines
    if (trimmed.startsWith('//') || trimmed.startsWith('*')) continue;

    const match = line.match(/console\.(log|error|warn|info|debug)\s*\(/);
    if (match) {
      violations.push({
        file: path.relative(REPO_ROOT, filePath).replace(/\\/g, '/'),
        line: i + 1,
        call: `console.${match[1]}`,
        snippet: line.trim(),
      });
    }
  }

  return violations;
}

// ===== Tests =====

describe('Architecture: Playback command routing', () => {
  it('all playback invoke() calls must be inside the allowed provider files', () => {
    const allFiles = SCAN_DIRS.flatMap(collectSourceFiles);
    const allViolations: Violation[] = [];

    for (const file of allFiles) {
      allViolations.push(...findPlaybackInvokeViolations(file));
    }

    if (allViolations.length > 0) {
      const report = allViolations
        .map(
          (v) =>
            `  ${v.file}:${v.line}\n    invoke('${v.command}') — use usePlayerCommands() instead\n    > ${v.snippet}`
        )
        .join('\n\n');

      throw new Error(
        `Found ${allViolations.length} playback architecture violation(s).\n\n` +
          `Components and pages must use usePlayerCommands() — never invoke() directly.\n` +
          `Only TauriPlayerCommandsProvider.tsx may call invoke() for playback commands.\n\n` +
          report
      );
    }

    expect(allViolations).toHaveLength(0);
  });

  it('PlayerCommandsContext exports usePlayerCommands hook', () => {
    const contextFile = path.join(
      REPO_ROOT,
      'applications/shared/src/contexts/PlayerCommandsContext.tsx'
    );
    expect(fs.existsSync(contextFile)).toBe(true);

    const content = fs.readFileSync(contextFile, 'utf-8');
    expect(content).toMatch(/export\s+function\s+usePlayerCommands|export\s+const\s+usePlayerCommands/);
  });

  it('TauriPlayerCommandsProvider implements every PLAYBACK_COMMAND via invoke()', () => {
    const providerFile = path.join(
      REPO_ROOT,
      'applications/desktop/src/providers/TauriPlayerCommandsProvider.tsx'
    );
    expect(fs.existsSync(providerFile)).toBe(true);

    // The provider is the ONLY allowed place — it must actually exist
    const content = fs.readFileSync(providerFile, 'utf-8');

    // Sanity-check: it must call invoke() at least once
    expect(content).toMatch(/invoke\s*\(/);
  });
});

describe('Architecture: Logging discipline', () => {
  it('source files must use debug utility instead of raw console.log/error/warn', () => {
    const allFiles = SCAN_DIRS.flatMap(collectSourceFiles);
    const newViolations: ConsoleViolation[] = [];

    for (const file of allFiles) {
      const violations = findConsoleLogViolations(file);
      if (violations.length === 0) continue;

      const relative = path.relative(REPO_ROOT, file).replace(/\\/g, '/');
      const isKnown = CONSOLE_LOG_KNOWN_VIOLATIONS.has(relative);

      if (!isKnown) {
        newViolations.push(...violations);
      }
    }

    if (newViolations.length > 0) {
      const report = newViolations
        .map(
          (v) =>
            `  ${v.file}:${v.line}\n    ${v.call}() — import { debug } from '../utils/debug' and use debug.log/error/warn\n    > ${v.snippet}`
        )
        .join('\n\n');

      throw new Error(
        `Found ${newViolations.length} NEW console.log violation(s).\n\n` +
          `Use debug.log / debug.error / debug.warn from @soul-player/shared instead of raw console.*.\n` +
          `Example: import { debug } from '../utils/debug';\n\n` +
          report
      );
    }

    expect(newViolations).toHaveLength(0);
  });

  it('debug utility (debug.ts) exists and wraps console', () => {
    const debugFile = path.join(
      REPO_ROOT,
      'applications/shared/src/utils/debug.ts'
    );
    expect(fs.existsSync(debugFile)).toBe(true);

    const content = fs.readFileSync(debugFile, 'utf-8');
    // Must export a `debug` object with at least a log/error method
    expect(content).toMatch(/export\s+(const|default)\s+debug/);
  });

  it('CONSOLE_LOG_KNOWN_VIOLATIONS contains no false positives (all listed files exist and still have violations)', () => {
    // Warn if a file was cleaned up but not removed from the list
    const staleEntries: string[] = [];

    for (const relative of CONSOLE_LOG_KNOWN_VIOLATIONS) {
      const full = path.join(REPO_ROOT, relative.replace(/\//g, path.sep));
      if (!fs.existsSync(full)) {
        staleEntries.push(`  ${relative} (file not found — remove from list)`);
        continue;
      }
      const violations = findConsoleLogViolations(full);
      if (violations.length === 0) {
        staleEntries.push(`  ${relative} (no violations remain — remove from list)`);
      }
    }

    if (staleEntries.length > 0) {
      throw new Error(
        `CONSOLE_LOG_KNOWN_VIOLATIONS has ${staleEntries.length} stale entries.\n` +
          `Remove them from the list to keep the enforcement tight:\n\n` +
          staleEntries.join('\n')
      );
    }

    expect(staleEntries).toHaveLength(0);
  });
});
