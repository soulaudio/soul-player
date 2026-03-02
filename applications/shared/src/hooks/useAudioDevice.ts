import { useState, useEffect, useCallback } from 'react'
import { useBackend } from '../contexts/BackendContext'
import type { AudioBackend, AudioDevice } from '../contexts/BackendContext'
import { debug } from '../utils/debug'

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
 *
 * 1. Load ordering: getCurrentAudioDevice() is called first so we always
 *    load devices for the REAL active backend — not stale React initial state
 *    (which was always 'default', causing WASAPI devices to show when ASIO was saved).
 *
 * 2. Backend forwarded correctly: switchDevice() always receives and forwards
 *    the backend from the clicked device, never from a stale closure value.
 *
 * Usage in settings page (shows one backend at a time):
 *   const { backends, devices, currentDevice, isLoading, switchDevice, loadBackend } = useAudioDevice()
 *   - call loadBackend(backend) when the user changes the backend picker
 *   - pass devices directly to DeviceSelector (variant="list")
 *
 * Usage in sidebar dropdown (shows all backends grouped):
 *   const { backends, devices, currentDevice, isLoading, switchDevice, loadAll } = useAudioDevice()
 *   - call loadAll() when the dropdown opens (onLoadDevices)
 *   - pass devices directly to DeviceSelector (variant="dropdown")
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
      // Fix for bug #1: load current device FIRST so we know the real active backend.
      // Run currentDevice and backends in parallel since they don't depend on each other.
      const [current, backendList] = await Promise.all([
        backend.getCurrentAudioDevice(),
        backend.getAudioBackends(),
      ])
      setCurrentDevice(current)
      setBackends(backendList)

      // Load devices for the ACTUAL active backend, not stale React state.
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

  /**
   * Load devices for a specific backend without switching the active device.
   * Called from the settings page backend picker so the list updates immediately.
   */
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
          try {
            const deviceList = await backend.getAudioDevices(b.backend)
            map.set(b.backend, deviceList)
          } catch (err) {
            // Backend enumeration failed — skip this backend but continue loading others.
            // This prevents a single problematic backend (e.g. ASIO driver crash) from
            // blocking all other backends from appearing in the dropdown.
            debug.warn(`[useAudioDevice] Failed to enumerate ${b.backend} devices:`, err)
          }
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
   * After switching, re-queries Rust for the actual current device (ground truth).
   */
  const switchDevice = useCallback(async (backendStr: string, deviceName: string) => {
    await backend.setAudioDevice(backendStr, deviceName)
    // Re-query Rust for the actual current device — don't trust stale React state.
    const current = await backend.getCurrentAudioDevice()
    setCurrentDevice(current)
  }, [backend])

  return { backends, devices, currentDevice, isLoading, switchDevice, loadBackend, loadAll, reload }
}
