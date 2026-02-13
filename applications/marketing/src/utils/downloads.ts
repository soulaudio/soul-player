export type Platform = 'windows' | 'macos' | 'linux' | 'unknown'

export interface DownloadConfig {
  filePattern: string
  displayName: string
  description: string
  recommended?: boolean
}

/**
 * Primary download configurations for each platform
 * These are the recommended installers shown in the main download button
 */
export const DOWNLOAD_CONFIGS: Record<Exclude<Platform, 'unknown'>, DownloadConfig> = {
  windows: {
    filePattern: 'Soul.Player_{version}_x64-setup.exe',
    displayName: 'Windows Installer',
    description: 'For Windows 10/11 (x64)',
    recommended: true,
  },
  macos: {
    filePattern: 'Soul.Player_{version}_aarch64.dmg',
    displayName: 'macOS Disk Image (Apple Silicon)',
    description: 'For Apple Silicon (M1/M2/M3/M4)',
    recommended: true,
  },
  linux: {
    filePattern: 'soul-player_{version}_x86_64.AppImage',
    displayName: 'Linux AppImage',
    description: 'Universal Linux package. No installation required.',
    recommended: true,
  },
}

export interface LinuxDownload {
  id: string
  filePattern: string
  displayName: string
  description: string
  installCommand?: string
  isAur?: boolean
}

/**
 * All available Linux download formats
 * Matches the structure from .github/release-config.json
 */
export const LINUX_DOWNLOADS: LinuxDownload[] = [
  {
    id: 'appimage',
    filePattern: 'soul-player_{version}_x86_64.AppImage',
    displayName: 'AppImage (Universal)',
    description: 'Works on all Linux distributions. No installation required.',
  },
  {
    id: 'flatpak',
    filePattern: 'io.github.soulaudio.SoulPlayer_{version}_x86_64.flatpak',
    displayName: 'Flatpak',
    description: 'Sandboxed universal package. Install: flatpak install <file>',
  },
  {
    id: 'deb',
    filePattern: 'Soul.Player_{version}_amd64.deb',
    displayName: 'Debian/Ubuntu (.deb)',
    description: 'For Debian, Ubuntu, Linux Mint, Pop!_OS',
  },
  {
    id: 'rpm',
    filePattern: 'Soul.Player-{version}-1.x86_64.rpm',
    displayName: 'Fedora/RHEL (.rpm)',
    description: 'For Fedora, RHEL, CentOS, openSUSE',
  },
  {
    id: 'aur',
    filePattern: '',
    displayName: 'Arch Linux (AUR)',
    description: 'Install via your favorite AUR helper (yay, paru, etc.)',
    installCommand: 'yay -S soul-player',
    isAur: true,
  },
]

export interface AlternateDownload {
  platform: Exclude<Platform, 'unknown'>
  filePattern: string
  label: string
  description: string
}

/**
 * Alternate download options for Windows and macOS
 * Note: macOS Intel is now included in the macOS download modal instead
 */
export const ALTERNATE_DOWNLOADS: AlternateDownload[] = []

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
 * Detect the current platform from user agent
 */
export function detectPlatform(): Platform {
  if (typeof window === 'undefined') return 'unknown'

  const ua = window.navigator.userAgent.toLowerCase()

  if (ua.includes('win')) return 'windows'
  if (ua.includes('mac')) return 'macos'
  if (ua.includes('linux')) return 'linux'

  return 'unknown'
}
