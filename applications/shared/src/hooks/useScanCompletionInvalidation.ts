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
 * - Rust importer must emit 'import-complete' event after import finishes
 *
 * @see applications/desktop/src-tauri/src/scanner.rs
 * @see applications/desktop/src-tauri/src/import.rs
 */

import { useEffect, useState } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { invalidateAfterFileScan } from './queries/invalidationHelpers'
import { debug } from '../utils/debug'

// ----------------------------------------------------------------
// Platform detection: is the app running inside Tauri?
// Tauri injects __TAURI_INTERNALS__ when running as a native app.
// This avoids depending on window.__TAURI__ (which requires the
// non-default withGlobalTauri config option).
// ----------------------------------------------------------------
function isRunningInTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}

/**
 * Hook that listens for library scan completion and import completion,
 * then invalidates caches.
 *
 * Platform Support:
 * - Desktop (Tauri): Listens to Tauri events via @tauri-apps/api/event
 * - Web/Demo: No-op (no file scanning in web mode)
 *
 * Performance:
 * - Uses broad invalidation (all albums, artists, tracks, genres)
 * - Only triggers once per scan/import completion
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
    // Only set up listeners in the Tauri desktop app.
    if (!isRunningInTauri()) {
      debug.log('[useScanCompletionInvalidation] Not running in Tauri, skipping event listeners')
      return
    }

    let unlistenScan: (() => void) | null = null
    let unlistenImport: (() => void) | null = null

    const setupListeners = async () => {
      try {
        const { listen } = await import('@tauri-apps/api/event')

        unlistenScan = await listen('scan-complete', (event) => {
          debug.log('[useScanCompletionInvalidation] Scan completed, invalidating library caches', event.payload)
          invalidateAfterFileScan(queryClient)
          debug.log('[useScanCompletionInvalidation] Cache invalidation complete')
        })

        // Also listen for import-complete — import_directory uses a separate
        // pipeline (import-progress / import-complete) that does NOT fire
        // scan-started/scan-complete events.
        unlistenImport = await listen('import-complete', (event) => {
          debug.log('[useScanCompletionInvalidation] Import completed, invalidating library caches', event.payload)
          invalidateAfterFileScan(queryClient)
          debug.log('[useScanCompletionInvalidation] Cache invalidation complete')
        })

        debug.log('[useScanCompletionInvalidation] Scan and import listeners registered')
      } catch (error) {
        debug.error('[useScanCompletionInvalidation] Failed to register event listeners:', error)
      }
    }

    void setupListeners()

    return () => {
      unlistenScan?.()
      unlistenImport?.()
      debug.log('[useScanCompletionInvalidation] Event listeners unregistered')
    }
  }, [queryClient])
}

/**
 * Hook for listening to scan progress updates (optional, for progress UI).
 *
 * Includes debounced cache invalidation during scanning to show new items
 * without overwhelming React Query with constant refetches.
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
  const [isDirty, setIsDirty] = useState(false)

  // Debounced invalidation during scanning (every 2 seconds max)
  useEffect(() => {
    if (!isDirty || !isScanning) return

    const timeoutId = setTimeout(() => {
      debug.log('[useScanProgress] Debounced invalidation - refreshing library data')
      invalidateAfterFileScan(queryClient)
      setIsDirty(false)
    }, 2000) // 2 second debounce

    return () => clearTimeout(timeoutId)
  }, [isDirty, isScanning, queryClient])

  useEffect(() => {
    if (!isRunningInTauri()) {
      return
    }

    let unlistenProgress: (() => void) | null = null
    let unlistenStart: (() => void) | null = null
    let unlistenComplete: (() => void) | null = null

    const setupListeners = async () => {
      try {
        const { listen } = await import('@tauri-apps/api/event')

        // Listen for scan start
        unlistenStart = await listen('scan-started', () => {
          setIsScanning(true)
          setProgress(0)
          setIsDirty(false)
          debug.log('[useScanProgress] Scan started')
        })

        // Listen for progress updates
        unlistenProgress = await listen<{ processed: number; total: number }>(
          'scan-progress',
          (event) => {
            const { processed, total } = event.payload
            const progressPercent = total > 0 ? (processed / total) * 100 : 0
            setProgress(progressPercent)

            // Mark as dirty on every progress update
            // The debounced effect above will handle actual invalidation
            setIsDirty(true)

            debug.log(`[useScanProgress] Progress: ${processed}/${total} (${progressPercent.toFixed(1)}%)`)
          }
        )

        // Listen for completion
        unlistenComplete = await listen('scan-complete', () => {
          setIsScanning(false)
          setProgress(100)
          debug.log('[useScanProgress] Scan complete')

          // Final invalidation on completion (immediate, not debounced)
          invalidateAfterFileScan(queryClient)

          // Reset after a delay
          setTimeout(() => {
            setProgress(0)
            setIsDirty(false)
          }, 2000)
        })
      } catch (error) {
        debug.error('[useScanProgress] Failed to register progress listeners:', error)
      }
    }

    void setupListeners()

    return () => {
      unlistenProgress?.()
      unlistenStart?.()
      unlistenComplete?.()
    }
  }, [queryClient])

  return { progress, isScanning, isDirty }
}

/**
 * Type declaration for Tauri internals (subset needed for Tauri detection).
 */
declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown
  }
}
