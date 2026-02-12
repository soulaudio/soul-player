/**
 * Scan Completion Invalidation Hook
 *
 * Listens for library scan completion events and automatically invalidates
 * all library-related caches. This ensures fresh imports appear in the UI
 * without requiring manual refresh.
 *
 * Usage: Call once at app root level (e.g., in App.tsx)
 *
 * ```typescript
 * export function App() {
 *   useScanCompletionInvalidation() // Add this line
 *   // ... rest of app
 * }
 * ```
 *
 * Backend Requirements:
 * - Rust scanner must emit 'scan-complete' event after scan finishes
 * - Event payload can be empty or include scan statistics
 *
 * @see applications/desktop/src-tauri/src/scanner.rs
 */

import { useEffect, useState } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { invalidateAfterFileScan } from './queries/invalidationHelpers'
import { debug } from '../utils/debug'

/**
 * Hook that listens for library scan completion and invalidates caches.
 *
 * Platform Support:
 * - Desktop (Tauri): Listens to Tauri events
 * - Web/Demo: No-op (no file scanning in web mode)
 *
 * Performance:
 * - Uses broad invalidation (all albums, artists, tracks, genres)
 * - Only triggers once per scan completion
 * - Background refetch doesn't block UI
 *
 * @example
 * ```typescript
 * // In App.tsx root component
 * import { useScanCompletionInvalidation } from '@soul-player/shared'
 *
 * export function App() {
 *   useScanCompletionInvalidation()
 *   return <YourApp />
 * }
 * ```
 */
export function useScanCompletionInvalidation(): void {
  const queryClient = useQueryClient()

  useEffect(() => {
    // Only set up listener if Tauri is available (desktop app)
    if (typeof window === 'undefined' || !window.__TAURI__) {
      debug.log('[useScanCompletionInvalidation] Tauri not available, skipping scan listener')
      return
    }

    let unlisten: (() => void) | null = null

    // Set up Tauri event listener
    const setupListener = async () => {
      try {
        if (!window.__TAURI__) {
          debug.log('[useScanCompletionInvalidation] Not running in Tauri, skipping listener setup')
          return
        }
        const { listen } = window.__TAURI__.event

        unlisten = await listen('scan-complete', (event) => {
          debug.log('[useScanCompletionInvalidation] Scan completed, invalidating library caches', event.payload)

          // Invalidate all library-related queries
          invalidateAfterFileScan(queryClient)

          debug.log('[useScanCompletionInvalidation] Cache invalidation complete')
        })

        debug.log('[useScanCompletionInvalidation] Scan listener registered')
      } catch (error) {
        debug.error('[useScanCompletionInvalidation] Failed to register scan listener:', error)
      }
    }

    setupListener()

    // Cleanup: unregister listener on unmount
    return () => {
      if (unlisten) {
        unlisten()
        debug.log('[useScanCompletionInvalidation] Scan listener unregistered')
      }
    }
  }, [queryClient])
}

/**
 * Hook for listening to scan progress updates (optional, for progress UI).
 *
 * Usage:
 * ```typescript
 * const { progress, isScanning } = useScanProgress()
 * return <ProgressBar value={progress} visible={isScanning} />
 * ```
 *
 * @returns Scan progress state
 */
export function useScanProgress() {
  const queryClient = useQueryClient()
  const [progress, setProgress] = useState(0)
  const [isScanning, setIsScanning] = useState(false)

  useEffect(() => {
    if (typeof window === 'undefined' || !window.__TAURI__) {
      return
    }

    let unlistenProgress: (() => void) | null = null
    let unlistenStart: (() => void) | null = null
    let unlistenComplete: (() => void) | null = null

    const setupListeners = async () => {
      try {
        if (!window.__TAURI__) return
        const { listen } = window.__TAURI__.event

        // Listen for scan start
        unlistenStart = await listen('scan-started', () => {
          setIsScanning(true)
          setProgress(0)
          debug.log('[useScanProgress] Scan started')
        })

        // Listen for progress updates
        unlistenProgress = await listen<{ processed: number; total: number }>(
          'scan-progress',
          (event) => {
            const { processed, total } = event.payload
            const progressPercent = total > 0 ? (processed / total) * 100 : 0
            setProgress(progressPercent)
            debug.log(`[useScanProgress] Progress: ${processed}/${total} (${progressPercent.toFixed(1)}%)`)
          }
        )

        // Listen for completion
        unlistenComplete = await listen('scan-complete', () => {
          setIsScanning(false)
          setProgress(100)
          debug.log('[useScanProgress] Scan complete')

          // Reset after a delay
          setTimeout(() => {
            setProgress(0)
          }, 2000)
        })
      } catch (error) {
        debug.error('[useScanProgress] Failed to register progress listeners:', error)
      }
    }

    setupListeners()

    return () => {
      unlistenProgress?.()
      unlistenStart?.()
      unlistenComplete?.()
    }
  }, [queryClient])

  return { progress, isScanning }
}

/**
 * Type declarations for Tauri events (if not in @tauri-apps/api types)
 */
declare global {
  interface Window {
    __TAURI__?: {
      event: {
        listen: <T = unknown>(
          event: string,
          handler: (event: { event: string; payload: T }) => void
        ) => Promise<() => void>
      }
    }
  }
}
