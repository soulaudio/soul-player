import { useState, useEffect } from 'react'
import { fetchLatestRelease, type ReleaseInfo } from '@/utils/github'

export interface UseGitHubReleaseResult {
  version: string | null
  isLoading: boolean
  error: Error | null
  release: ReleaseInfo | null
  refetch: () => Promise<void>
}

/**
 * React hook to fetch and manage GitHub release information
 *
 * Features:
 * - Fetches latest release on mount
 * - Caches results in sessionStorage (5-minute TTL)
 * - Provides loading and error states
 * - Gracefully handles API failures
 *
 * @returns Release information and state management
 */
export function useGitHubRelease(): UseGitHubReleaseResult {
  const [release, setRelease] = useState<ReleaseInfo | null>(null)
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState<Error | null>(null)

  const fetchRelease = async () => {
    try {
      setIsLoading(true)
      setError(null)

      const releaseData = await fetchLatestRelease()

      if (releaseData) {
        setRelease(releaseData)
      } else {
        // API returned null (rate limit, 404, etc.) - not an error
        setRelease(null)
      }
    } catch (err) {
      const error = err instanceof Error ? err : new Error('Unknown error')
      setError(error)
      setRelease(null)
      console.error('[useGitHubRelease] Error fetching release:', error)
    } finally {
      setIsLoading(false)
    }
  }

  useEffect(() => {
    fetchRelease()
  }, [])

  return {
    version: release?.version || null,
    isLoading,
    error,
    release,
    refetch: fetchRelease,
  }
}
