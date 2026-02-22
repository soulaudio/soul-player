# Audio Device Selection — Unified Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace duplicated, buggy audio device selection code with a single `useAudioDevice` hook used by both the settings page and the sidebar player panel.

**Architecture:** Add `getCurrentAudioDevice` to `BackendContext`, create `useAudioDevice` hook that fixes the load-ordering bug (loads current device from Rust first, then loads devices for the *actual* backend — not stale React state). Remove all duplicate audio device methods from `PlayerCommandsContext` and `TauriPlayerCommandsProvider`. Both `AudioSettingsPage` and `PlayerPanel` consume the hook. Add Rust E2E persistence test for device selection.

**Tech Stack:** React hooks, TypeScript, Tauri `invoke`, SQLite via `soul_storage::settings`, `tokio::test`

---

## Bug summary (what we are fixing)

1. **Init ordering bug**: `AudioSettingsPage` reads `settings.backend` (always `'default'` on first render) before saved settings load → WASAPI devices shown when ASIO was saved.
2. **Backend ignored**: `onSwitchDevice={(_backend, deviceName) => …}` — the `_backend` is dropped; `settings.backend` (stale closure) is used instead.
3. **Duplicate implementation**: audio device methods live in both `BackendContext` and `PlayerCommandsContext`; two providers implement the same Tauri invocations.
4. **`getCurrentAudioDevice` missing from BackendContext**: only in `PlayerCommandsContext`, so `AudioSettingsPage` can't query Rust for the real current device.
5. **Wrong selection in sidebar**: `currentDevice` comes from a stale React state update after `switchDevice`, not from re-querying Rust, so the wrong device can briefly show as selected.

---

## Task 1: Add `getCurrentAudioDevice` to BackendContext

**Files:**
- Modify: `applications/shared/src/contexts/BackendContext.tsx` (line ~349)

**Step 1: Add to interface**

In `BackendInterface`, after line `setAudioDevice: …`, add:

```typescript
  getCurrentAudioDevice: () => Promise<AudioDevice | null>
```

**Step 2: Verify TypeScript now errors on all providers**

```bash
cargo xtask check typescript 2>&1 | grep "getCurrentAudioDevice"
```

Expected: errors in TauriBackendProvider, MockBackendProvider, ServerBackendProvider (not yet implemented).

**Step 3: Commit**

```bash
git add applications/shared/src/contexts/BackendContext.tsx
git commit -m "feat(audio): add getCurrentAudioDevice to BackendInterface"
```

---

## Task 2: Implement `getCurrentAudioDevice` in all providers

**Files:**
- Modify: `applications/desktop/src/providers/TauriBackendProvider.tsx` (after `setAudioDevice`)
- Modify: `applications/shared/src/providers/MockBackendProvider.tsx` (after `setAudioDevice` stub)
- Modify: `applications/shared/src/providers/ServerBackendProvider.tsx` (after `setAudioDevice` stub)

**Step 1: TauriBackendProvider** — add after `setAudioDevice`:

```typescript
async getCurrentAudioDevice() {
  try {
    return await invoke('get_current_audio_device')
  } catch {
    return null
  }
},
```

**Step 2: MockBackendProvider** — add after `setAudioDevice` stub:

```typescript
async getCurrentAudioDevice() {
  return null
},
```

**Step 3: ServerBackendProvider** — add after `setAudioDevice` stub:

```typescript
async getCurrentAudioDevice() {
  return null
},
```

**Step 4: Verify TypeScript clean**

```bash
cargo xtask check typescript 2>&1 | grep -i error
```

Expected: 0 errors related to `getCurrentAudioDevice`.

**Step 5: Commit**

```bash
git add applications/desktop/src/providers/TauriBackendProvider.tsx \
        applications/shared/src/providers/MockBackendProvider.tsx \
        applications/shared/src/providers/ServerBackendProvider.tsx
git commit -m "feat(audio): implement getCurrentAudioDevice in all providers"
```

---

## Task 3: Remove duplicate audio device methods from PlayerCommandsContext

**Files:**
- Modify: `applications/shared/src/contexts/PlayerCommandsContext.tsx` (lines 91-95)
- Modify: `applications/desktop/src/providers/TauriPlayerCommandsProvider.tsx` (lines 628-643)

**Step 1: Remove from PlayerCommandsContext interface**

Delete these 4 lines from `PlayerCommandsInterface`:

```typescript
  // Audio device management (Desktop only - optional)
  getCurrentAudioDevice?: () => Promise<any>;
  getAudioBackends?: () => Promise<any[]>;
  getAudioDevices?: (backend: string) => Promise<any[]>;
  setAudioDevice?: (backend: string, deviceName: string) => Promise<void>;
```

**Step 2: Remove from TauriPlayerCommandsProvider**

Delete the implementation block (approximately lines 628-643):

```typescript
      // Audio device management (Desktop only)
      async getCurrentAudioDevice() {
        return await invoke('get_current_audio_device');
      },

      async getAudioBackends() {
        return await invoke('get_audio_backends');
      },

      async getAudioDevices(backend: string) {
        return await invoke('get_audio_devices', { backendStr: backend });
      },

      async setAudioDevice(backend: string, deviceName: string) {
        await invoke('set_audio_device', { backendStr: backend, deviceName });
      },
```

**Step 3: Verify TypeScript clean**

```bash
cargo xtask check typescript 2>&1 | grep -i error
```

Expected: 0 errors (PlayerPanel will break next — that's expected, fixed in Task 5).

**Step 4: Commit**

```bash
git add applications/shared/src/contexts/PlayerCommandsContext.tsx \
        applications/desktop/src/providers/TauriPlayerCommandsProvider.tsx
git commit -m "refactor(audio): remove duplicate device methods from PlayerCommandsContext"
```

---

## Task 4: Create `useAudioDevice` hook

**Files:**
- Create: `applications/shared/src/hooks/useAudioDevice.ts`

**Step 1: Write the hook**

```typescript
import { useState, useEffect, useCallback } from 'react'
import { useBackend } from '../contexts/BackendContext'
import type { AudioBackend, AudioDevice } from '../contexts/BackendContext'

export interface UseAudioDeviceResult {
  backends: AudioBackend[]
  devices: Map<string, AudioDevice[]>
  currentDevice: AudioDevice | null
  isLoading: boolean
  switchDevice: (backend: string, deviceName: string) => Promise<void>
  loadBackend: (backend: string) => Promise<void>
  loadAll: () => Promise<void>
  reload: () => Promise<void>
}

/**
 * Unified audio device selection hook.
 *
 * Fixes two bugs present in the old per-component approach:
 * 1. Load ordering: getCurrentAudioDevice() is called first so we always
 *    load devices for the REAL active backend (not stale React initial state).
 * 2. Backend ignored: switchDevice() always receives and forwards the backend
 *    from the clicked device, never from stale closure state.
 *
 * Usage:
 *   const { backends, devices, currentDevice, isLoading, switchDevice, loadBackend, loadAll } = useAudioDevice()
 *
 * - Settings page: call loadBackend(backend) when user changes the backend picker.
 * - Sidebar dropdown: call loadAll() when the dropdown opens.
 */
export function useAudioDevice(hasRealDevices = true): UseAudioDeviceResult {
  const backend = useBackend()
  const [backends, setBackends] = useState<AudioBackend[]>([])
  const [devices, setDevices] = useState<Map<string, AudioDevice[]>>(new Map())
  const [currentDevice, setCurrentDevice] = useState<AudioDevice | null>(null)
  const [isLoading, setIsLoading] = useState(false)

  const reload = useCallback(async () => {
    if (!hasRealDevices) return
    setIsLoading(true)
    try {
      // Fix for bug #1: load current device FIRST so we know the real backend.
      // Do backends and currentDevice in parallel since they don't depend on each other.
      const [current, backendList] = await Promise.all([
        backend.getCurrentAudioDevice(),
        backend.getAudioBackends(),
      ])
      setCurrentDevice(current)
      setBackends(backendList)

      // Load devices for the ACTUAL active backend, not the stale React state.
      const activeBackend = current?.backend ?? 'default'
      const deviceList = await backend.getAudioDevices(activeBackend)
      setDevices(new Map([[activeBackend, deviceList]]))
    } finally {
      setIsLoading(false)
    }
  }, [backend, hasRealDevices])

  useEffect(() => {
    reload()
  }, [reload])

  /** Load devices for a specific backend (called from settings page backend picker). */
  const loadBackend = useCallback(async (backendStr: string) => {
    setIsLoading(true)
    try {
      const deviceList = await backend.getAudioDevices(backendStr)
      setDevices(new Map([[backendStr, deviceList]]))
    } finally {
      setIsLoading(false)
    }
  }, [backend])

  /**
   * Load devices for ALL available backends.
   * Called when the sidebar dropdown opens so it can show all backends in groups.
   */
  const loadAll = useCallback(async () => {
    if (!hasRealDevices) return
    setIsLoading(true)
    try {
      const map = new Map<string, AudioDevice[]>()
      for (const b of backends) {
        if (b.available) {
          const deviceList = await backend.getAudioDevices(b.backend)
          map.set(b.backend, deviceList)
        }
      }
      setDevices(map)
    } finally {
      setIsLoading(false)
    }
  }, [backend, backends, hasRealDevices])

  /**
   * Switch the active audio device.
   * Fix for bug #2: backend is always taken from the argument, never from stale state.
   */
  const switchDevice = useCallback(async (backendStr: string, deviceName: string) => {
    await backend.setAudioDevice(backendStr, deviceName)
    // Re-query Rust for the actual current device (ground truth).
    const current = await backend.getCurrentAudioDevice()
    setCurrentDevice(current)
  }, [backend])

  return { backends, devices, currentDevice, isLoading, switchDevice, loadBackend, loadAll, reload }
}
```

**Step 2: Export from shared index**

Find the hooks export barrel (likely `applications/shared/src/index.ts` or `hooks/index.ts`).
Add: `export { useAudioDevice } from './hooks/useAudioDevice'`

If no barrel exists, skip — import directly in consumer files.

**Step 3: Verify it compiles**

```bash
cargo xtask check typescript 2>&1 | grep -i error
```

**Step 4: Commit**

```bash
git add applications/shared/src/hooks/useAudioDevice.ts
git commit -m "feat(audio): add useAudioDevice shared hook with load-order fix"
```

---

## Task 5: Refactor PlayerPanel to use useAudioDevice

**Files:**
- Modify: `applications/shared/src/components/sidebar/PlayerPanel.tsx`

**Step 1: Replace device state with hook**

Remove these state variables and functions:
```typescript
const [currentDevice, setCurrentDevice] = useState<AudioDevice | null>(null);
const [backends, setBackends] = useState<AudioBackend[]>([]);
const [devices, setDevices] = useState<Map<string, AudioDevice[]>>(new Map());
const [isLoadingDevices, setIsLoadingDevices] = useState(false);

const loadCurrentDevice = async () => { … }
const loadDevices = async () => { … }
const switchDevice = async (backend: string, deviceName: string) => { … }
```

Remove the `useEffect` that calls `loadCurrentDevice()`.

Add at the top of `PlayerPanel` (after existing hooks):
```typescript
const {
  backends,
  devices,
  currentDevice,
  isLoading: isLoadingDevices,
  switchDevice,
  loadAll,
} = useAudioDevice(hasRealDevices)
```

**Step 2: Update DeviceSelector usage**

Replace `onLoadDevices={loadDevices}` with `onLoadDevices={loadAll}` and `onSwitchDevice={switchDevice}`.

The full DeviceSelector block becomes:

```tsx
<DeviceSelector
  currentDevice={currentDevice}
  backends={backends}
  devices={devices}
  isLoadingDevices={isLoadingDevices}
  hasRealDevices={hasRealDevices}
  onLoadDevices={loadAll}
  onSwitchDevice={switchDevice}
/>
```

**Step 3: Remove unused imports**

Remove `AudioDevice`, `AudioBackend` type imports if they were only used for the old state. Remove import of `debug` if no longer used. Remove import of `PlayerCommandsContext` audio device optional fields.

**Step 4: Verify TypeScript clean**

```bash
cargo xtask check typescript 2>&1 | grep -i error
```

**Step 5: Commit**

```bash
git add applications/shared/src/components/sidebar/PlayerPanel.tsx
git commit -m "refactor(audio): PlayerPanel uses useAudioDevice hook"
```

---

## Task 6: Refactor AudioSettingsPage to use useAudioDevice

**Files:**
- Modify: `applications/shared/src/components/settings/AudioSettingsPage.tsx`

**Step 1: Add hook to AudioSettingsDesktop**

At the top of `AudioSettingsDesktop`, add:

```typescript
const {
  backends,
  devices,
  currentDevice: activeDevice,
  isLoading: isLoadingDevices,
  switchDevice: switchAudioDevice,
  loadBackend,
} = useAudioDevice(true)
```

**Step 2: Remove device state from the component**

Remove from `AudioSettingsDesktop` state:
- `const [backends, setBackends] = useState<AudioBackend[]>([])`
- `const [devices, setDevices] = useState<Map<string, AudioDevice[]>>(new Map())`
- The device-loading parts of `loadAudioSettings`:
  ```typescript
  const backendsData = await backend.getAudioBackends()
  setBackends(backendsData)
  const currentBackend = settings.backend
  const devicesData = await backend.getAudioDevices(currentBackend)
  const deviceMap = new Map<string, AudioDevice[]>()
  deviceMap.set(currentBackend, devicesData)
  setDevices(deviceMap)
  ```

Keep `loadAudioSettings` but only for loading the non-device settings (DSP, resampling, volume leveling, etc.).

**Step 3: Fix handleBackendChange**

Replace:
```typescript
const handleBackendChange = async (selectedBackend: 'default' | 'asio' | 'jack') => {
  updateSettings({ backend: selectedBackend })
  try {
    const devicesData = await backend.getAudioDevices(selectedBackend)
    const deviceMap = new Map<string, AudioDevice[]>()
    deviceMap.set(selectedBackend, devicesData)
    setDevices(deviceMap)
  } catch (error) {
    debug.error('Failed to load devices:', error)
  }
}
```

With:
```typescript
const handleBackendChange = async (selectedBackend: 'default' | 'asio' | 'jack') => {
  updateSettings({ backend: selectedBackend })
  await loadBackend(selectedBackend)
}
```

**Step 4: Fix handleDeviceChange → replace with onSwitchDevice that takes (backend, deviceName)**

Remove:
```typescript
const handleDeviceChange = async (deviceName: string) => {
  updateSettings({ device_name: deviceName })
  try {
    await backend.setAudioDevice(settings.backend, deviceName)  // ← BUG: ignores backend
    showNotification('success', `Switched to audio device: ${deviceName}`)
  } catch (error) {
    debug.error('Failed to set audio device:', error)
    showNotification('error', `Failed to switch audio device: ${error}`)
  }
}
```

Add:
```typescript
const handleSwitchDevice = async (backendStr: string, deviceName: string) => {
  updateSettings({ backend: backendStr as 'default' | 'asio' | 'jack', device_name: deviceName })
  try {
    await switchAudioDevice(backendStr, deviceName)  // ← Fix: backend always passed
    showNotification('success', `Switched to audio device: ${deviceName}`)
  } catch (error) {
    debug.error('Failed to set audio device:', error)
    showNotification('error', `Failed to switch audio device: ${error}`)
  }
}
```

**Step 5: Update DeviceSelector usage**

Replace the DeviceSelector block:

```tsx
<DeviceSelector
  currentDevice={activeDevice}
  backends={backends}
  devices={devices}
  isLoadingDevices={isLoadingDevices}
  hasRealDevices={true}
  onLoadDevices={() => {}}
  onSwitchDevice={handleSwitchDevice}
  variant="list"
/>
```

Note: `currentDevice` is now from `activeDevice` (hook, Rust ground truth) not the synthetic object built from `settings.device_name`. This fixes the "no item selected on load" issue.

**Step 6: Sync backend picker with active device on load**

Add a `useEffect` to sync the `BackendSelector` display when the active device is first loaded:

```typescript
useEffect(() => {
  if (activeDevice?.backend && !loading) {
    // Sync the settings.backend with what's actually playing
    setSettings(prev => ({ ...prev, backend: activeDevice.backend as 'default' | 'asio' | 'jack' }))
  }
}, [activeDevice?.backend, loading])
```

**Step 7: Verify TypeScript clean**

```bash
cargo xtask check typescript 2>&1 | grep -i error
```

**Step 8: Commit**

```bash
git add applications/shared/src/components/settings/AudioSettingsPage.tsx
git commit -m "fix(audio): AudioSettingsPage uses useAudioDevice, fixes init-ordering and backend bugs"
```

---

## Task 7: Write E2E persistence test for device selection

**Files:**
- Create: `applications/desktop/src-tauri/tests/audio_device_selection_test.rs`

**Step 1: Write the test file**

```rust
//! Audio device selection persistence tests
//!
//! Verifies that device selection (backend + device name) persists across
//! simulated app restarts and is correctly isolated per user.

use serde_json::json;
use sqlx::SqlitePool;
use tempfile::TempDir;

const SETTING_OUTPUT_DEVICE: &str = "audio.output_device";

struct TestDb {
    db_path: std::path::PathBuf,
    _temp_dir: TempDir,
}

impl TestDb {
    async fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let db_path = temp_dir.path().join("test.db");

        let pool = Self::create_pool(&db_path).await;
        soul_storage::run_migrations(&pool)
            .await
            .expect("Failed to run migrations");

        let now = chrono::Utc::now().timestamp();
        sqlx::query("INSERT INTO users (id, name, created_at) VALUES (?, ?, ?)")
            .bind("1")
            .bind("Test User")
            .bind(now)
            .execute(&pool)
            .await
            .expect("Failed to create test user");

        pool.close().await;
        Self { db_path, _temp_dir: temp_dir }
    }

    async fn create_pool(db_path: &std::path::Path) -> SqlitePool {
        let db_url = if cfg!(windows) {
            format!("sqlite:///{}", db_path.to_str().unwrap().replace('\\', "/"))
        } else {
            format!("sqlite://{}", db_path.to_str().unwrap())
        };
        soul_storage::create_pool(&db_url)
            .await
            .expect("Failed to create pool")
    }

    async fn open(&self) -> SqlitePool {
        Self::create_pool(&self.db_path).await
    }

    async fn add_user(&self, pool: &SqlitePool, user_id: &str) {
        let now = chrono::Utc::now().timestamp();
        sqlx::query("INSERT INTO users (id, name, created_at) VALUES (?, ?, ?)")
            .bind(user_id)
            .bind(format!("User {}", user_id))
            .bind(now)
            .execute(pool)
            .await
            .expect("Failed to create user");
    }
}

/// WASAPI (default) device selection persists across restart
#[tokio::test]
async fn test_wasapi_device_persists_across_restart() {
    let db = TestDb::new().await;
    let user_id = "1";

    // Session 1: save WASAPI device
    {
        let pool = db.open().await;
        soul_storage::settings::set_setting(
            &pool,
            user_id,
            SETTING_OUTPUT_DEVICE,
            &json!({ "backend": "default", "device_name": "Speakers (Realtek Audio)" }),
        )
        .await
        .expect("Failed to save device");
        pool.close().await;
    }

    // Session 2: verify it survives restart
    {
        let pool = db.open().await;
        let saved = soul_storage::settings::get_setting(&pool, user_id, SETTING_OUTPUT_DEVICE)
            .await
            .expect("Failed to get setting")
            .expect("Setting should exist after restart");

        assert_eq!(saved["backend"].as_str().unwrap(), "default");
        assert_eq!(
            saved["device_name"].as_str().unwrap(),
            "Speakers (Realtek Audio)"
        );
        pool.close().await;
    }
}

/// ASIO device selection persists across restart
#[tokio::test]
async fn test_asio_device_persists_across_restart() {
    let db = TestDb::new().await;
    let user_id = "1";

    {
        let pool = db.open().await;
        soul_storage::settings::set_setting(
            &pool,
            user_id,
            SETTING_OUTPUT_DEVICE,
            &json!({ "backend": "asio", "device_name": "Focusrite USB ASIO" }),
        )
        .await
        .expect("Failed to save ASIO device");
        pool.close().await;
    }

    {
        let pool = db.open().await;
        let saved = soul_storage::settings::get_setting(&pool, user_id, SETTING_OUTPUT_DEVICE)
            .await
            .expect("Failed to get setting")
            .expect("ASIO setting should exist after restart");

        assert_eq!(saved["backend"].as_str().unwrap(), "asio", "Backend should be asio");
        assert_eq!(
            saved["device_name"].as_str().unwrap(),
            "Focusrite USB ASIO",
            "ASIO device name should survive restart"
        );
        pool.close().await;
    }
}

/// Switching device updates the stored setting
#[tokio::test]
async fn test_device_switch_overwrites_previous() {
    let db = TestDb::new().await;
    let user_id = "1";

    {
        let pool = db.open().await;
        // Start on WASAPI
        soul_storage::settings::set_setting(
            &pool,
            user_id,
            SETTING_OUTPUT_DEVICE,
            &json!({ "backend": "default", "device_name": "Speakers" }),
        )
        .await
        .expect("Failed to save initial device");

        // Switch to ASIO
        soul_storage::settings::set_setting(
            &pool,
            user_id,
            SETTING_OUTPUT_DEVICE,
            &json!({ "backend": "asio", "device_name": "ASIO4ALL v2" }),
        )
        .await
        .expect("Failed to save ASIO device");

        pool.close().await;
    }

    {
        let pool = db.open().await;
        let saved = soul_storage::settings::get_setting(&pool, user_id, SETTING_OUTPUT_DEVICE)
            .await
            .expect("Failed to get setting")
            .expect("Setting should exist");

        // Only the ASIO setting should be stored (upsert, not append)
        assert_eq!(saved["backend"].as_str().unwrap(), "asio", "Should have ASIO after switch");
        assert_eq!(saved["device_name"].as_str().unwrap(), "ASIO4ALL v2");
        pool.close().await;
    }
}

/// Two users have independent device settings
#[tokio::test]
async fn test_multi_user_device_isolation() {
    let db = TestDb::new().await;

    {
        let pool = db.open().await;
        db.add_user(&pool, "2").await;

        // User 1 uses WASAPI
        soul_storage::settings::set_setting(
            &pool,
            "1",
            SETTING_OUTPUT_DEVICE,
            &json!({ "backend": "default", "device_name": "Speakers" }),
        )
        .await
        .expect("Failed to save user 1 device");

        // User 2 uses ASIO
        soul_storage::settings::set_setting(
            &pool,
            "2",
            SETTING_OUTPUT_DEVICE,
            &json!({ "backend": "asio", "device_name": "ASIO4ALL v2" }),
        )
        .await
        .expect("Failed to save user 2 device");

        pool.close().await;
    }

    {
        let pool = db.open().await;

        let user1 = soul_storage::settings::get_setting(&pool, "1", SETTING_OUTPUT_DEVICE)
            .await
            .expect("DB error")
            .expect("User 1 setting should exist");
        let user2 = soul_storage::settings::get_setting(&pool, "2", SETTING_OUTPUT_DEVICE)
            .await
            .expect("DB error")
            .expect("User 2 setting should exist");

        assert_eq!(user1["backend"].as_str().unwrap(), "default", "User 1 should have WASAPI");
        assert_eq!(user2["backend"].as_str().unwrap(), "asio", "User 2 should have ASIO");
        assert_ne!(
            user1["device_name"].as_str().unwrap(),
            user2["device_name"].as_str().unwrap(),
            "Users should have different device names"
        );

        pool.close().await;
    }
}

/// No saved device — get_setting returns None (graceful absence)
#[tokio::test]
async fn test_no_saved_device_returns_none() {
    let db = TestDb::new().await;
    let pool = db.open().await;

    let result = soul_storage::settings::get_setting(&pool, "1", SETTING_OUTPUT_DEVICE)
        .await
        .expect("DB error");

    assert!(
        result.is_none(),
        "No device setting should return None, not error"
    );
    pool.close().await;
}
```

**Step 2: Run tests**

```bash
cargo test --test audio_device_selection_test -p soul-player-desktop 2>&1
```

Expected: all 5 tests pass.

**Step 3: Commit**

```bash
git add applications/desktop/src-tauri/tests/audio_device_selection_test.rs
git commit -m "test(audio): add E2E persistence tests for device selection"
```

---

## Task 8: Full check and final commit

**Step 1: Run all checks**

```bash
cargo xtask check precommit
```

Expected: all pass.

**Step 2: Run the new device selection tests**

```bash
cargo test --test audio_device_selection_test -p soul-player-desktop 2>&1
```

**Step 3: Final commit if any stragglers**

```bash
git add -p   # review any remaining changes
git commit -m "chore(audio): cleanup and verify unified device selection"
```

---

## Verification checklist

- [ ] TypeScript: 0 errors (`cargo xtask check typescript`)
- [ ] Rust tests: all pass (`cargo xtask check test`)
- [ ] Clippy: clean (`cargo xtask check clippy`)
- [ ] Sidebar dropdown: selecting ASIO Speakers shows checkmark ONLY on ASIO Speakers (not WASAPI Speakers)
- [ ] Settings page: on load with saved ASIO backend, device list shows ASIO devices (not WASAPI)
- [ ] Settings page: selecting a device in the list correctly switches that backend
- [ ] Settings page: backend picker selection matches active device on first load
- [ ] E2E tests: 5/5 pass
