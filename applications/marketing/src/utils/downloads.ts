export type Platform = 'windows' | 'macos' | 'linux' | 'unknown'

export interface DownloadConfig {
  filePattern: string
  displayName: string
  description: string
}

/**
 * Primary download configurations for each platform
 * These are the recommended installers shown in the main download button
 */
export const DOWNLOAD_CONFIGS: Record<Exclude<Platform, 'unknown'>, DownloadConfig> = {
  windows: {
    filePattern: 'soul-player-v{version}-x64.msi',
    displayName: 'Windows Installer',
    description: 'For Windows 10/11 (x64)',
  },
  macos: {
    filePattern: 'soul-player-v{version}-apple-silicon.dmg',
    displayName: 'macOS Installer',
    description: 'For Apple Silicon (M1/M2/M3/M4)',
  },
  linux: {
    filePattern: 'soul-player-v{version}.AppImage',
    displayName: 'Linux AppImage',
    description: 'Universal Linux package',
  },
}

export interface AlternateDownload {
  platform: Exclude<Platform, 'unknown'>
  filePattern: string
  label: string
  description: string
}

/**
 * Alternate download options shown in the "Other platforms" dropdown
 */
export const ALTERNATE_DOWNLOADS: AlternateDownload[] = [
  {
    platform: 'windows',
    filePattern: 'soul-player-v{version}-x64.exe',
    label: 'Windows NSIS',
    description: 'Portable installer for Windows',
  },
  {
    platform: 'macos',
    filePattern: 'soul-player-v{version}-intel.dmg',
    label: 'macOS Intel',
    description: 'For Intel-based Macs',
  },
  {
    platform: 'linux',
    filePattern: 'soul-player-v{version}-amd64.deb',
    label: 'Debian/Ubuntu',
    description: 'DEB package for Debian-based systems',
  },
  {
    platform: 'linux',
    filePattern: 'soul-player-v{version}.x86_64.rpm',
    label: 'Fedora/RHEL',
    description: 'RPM package for Red Hat-based systems',
  },
]

export interface DockerDownload {
  filePattern: string
  label: string
  description: string
  dockerImage: string
}

/**
 * Docker/Server download configuration
 */
export const DOCKER_DOWNLOAD: DockerDownload = {
  filePattern: 'soul-server-v{version}.tar.gz',
  label: 'Server (Docker)',
  description: 'Self-hosted server tarball',
  dockerImage: 'ghcr.io/soulaudio/soul-player/soul-server',
}

/**
 * Replace {version} placeholder in a file pattern with actual version
 */
export function fillVersionPattern(pattern: string, version: string): string {
  return pattern.replace('{version}', version)
}

/**
 * Get the recommended download config for a platform
 */
export function getDownloadConfigForPlatform(
  platform: Platform
): DownloadConfig | null {
  if (platform === 'unknown') {
    return null
  }
  return DOWNLOAD_CONFIGS[platform]
}

/**
 * Get alternate downloads for a specific platform
 */
export function getAlternateDownloadsForPlatform(
  platform: Platform
): AlternateDownload[] {
  if (platform === 'unknown') {
    return ALTERNATE_DOWNLOADS
  }
  return ALTERNATE_DOWNLOADS.filter((alt) => alt.platform !== platform)
}
