export const GITHUB_CONFIG = {
  owner: 'soulaudio',
  repo: 'soul-player',
}

export interface ReleaseAsset {
  name: string
  downloadUrl: string
  size: number
}

export interface ReleaseInfo {
  version: string
  tagName: string
  publishedAt: string
  assets: ReleaseAsset[]
}

interface CachedRelease {
  data: ReleaseInfo
  timestamp: number
}

const CACHE_KEY = 'soul-player-release-info'
const CACHE_TTL = 5 * 60 * 1000 // 5 minutes

/**
 * Fetch the latest release information from GitHub API
 * Uses sessionStorage caching to avoid rate limiting
 */
export async function fetchLatestRelease(): Promise<ReleaseInfo | null> {
  // Check cache first
  if (typeof window !== 'undefined') {
    const cached = sessionStorage.getItem(CACHE_KEY)
    if (cached) {
      try {
        const { data, timestamp }: CachedRelease = JSON.parse(cached)
        const age = Date.now() - timestamp
        if (age < CACHE_TTL) {
          console.log('[GitHub] Using cached release info')
          return data
        }
      } catch (error) {
        console.warn('[GitHub] Failed to parse cached release:', error)
        sessionStorage.removeItem(CACHE_KEY)
      }
    }
  }

  // Fetch from API
  const url = `https://api.github.com/repos/${GITHUB_CONFIG.owner}/${GITHUB_CONFIG.repo}/releases/latest`

  try {
    console.log('[GitHub] Fetching latest release from API')
    const response = await fetch(url, {
      headers: {
        Accept: 'application/vnd.github.v3+json',
      },
      // Don't cache in browser, we handle caching ourselves
      cache: 'no-store',
    })

    if (!response.ok) {
      if (response.status === 404) {
        console.warn('[GitHub] No releases found (404)')
        return null
      }
      if (response.status === 403) {
        console.warn('[GitHub] API rate limit exceeded (403)')
        return null
      }
      throw new Error(`GitHub API error: ${response.status} ${response.statusText}`)
    }

    const data = await response.json()

    const releaseInfo: ReleaseInfo = {
      version: data.tag_name.replace(/^v/, ''), // Strip leading 'v'
      tagName: data.tag_name,
      publishedAt: data.published_at,
      assets: data.assets.map((asset: any) => ({
        name: asset.name,
        downloadUrl: asset.browser_download_url,
        size: asset.size,
      })),
    }

    // Cache the result
    if (typeof window !== 'undefined') {
      const cacheData: CachedRelease = {
        data: releaseInfo,
        timestamp: Date.now(),
      }
      sessionStorage.setItem(CACHE_KEY, JSON.stringify(cacheData))
    }

    console.log(`[GitHub] Fetched release v${releaseInfo.version} with ${releaseInfo.assets.length} assets`)
    return releaseInfo
  } catch (error) {
    console.error('[GitHub] Failed to fetch release:', error)
    return null
  }
}

/**
 * Construct a direct download URL for a specific asset filename
 */
export function getDownloadUrl(filename: string): string {
  return `https://github.com/${GITHUB_CONFIG.owner}/${GITHUB_CONFIG.repo}/releases/latest/download/${filename}`
}

/**
 * Get the GitHub releases page URL (fallback when API fails)
 */
export function getReleasesPageUrl(): string {
  return `https://github.com/${GITHUB_CONFIG.owner}/${GITHUB_CONFIG.repo}/releases/latest`
}

/**
 * Get the repository URL
 */
export function getRepositoryUrl(): string {
  return `https://github.com/${GITHUB_CONFIG.owner}/${GITHUB_CONFIG.repo}`
}

/**
 * Clear the cached release info (useful for testing)
 */
export function clearReleaseCache(): void {
  if (typeof window !== 'undefined') {
    sessionStorage.removeItem(CACHE_KEY)
    console.log('[GitHub] Cache cleared')
  }
}
