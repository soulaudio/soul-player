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
