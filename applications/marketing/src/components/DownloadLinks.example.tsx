/**
 * Example: How to use release-config.json in the marketing page
 *
 * This shows how the marketing page can use the same config as CI tests,
 * ensuring download links are always in sync with tested artifacts.
 */

import releaseConfig from '../../../../.github/release-config.json';

// Type definitions matching release-config.json structure
interface ArtifactConfig {
  name: string;
  artifactName: string;
  path: string;
  displayName: string;
  platform: string;
  arch: string;
  recommended: boolean;
}

interface ReleaseConfig {
  version: string;
  repository: string;
  artifacts: {
    windows: Record<string, ArtifactConfig>;
    linux: Record<string, ArtifactConfig>;
    macos: Record<string, ArtifactConfig>;
  };
  downloadBaseUrl: string;
  latestReleaseUrl: string;
}

const config = releaseConfig as ReleaseConfig;

/**
 * Generate download URL for an artifact
 */
function getDownloadUrl(artifact: ArtifactConfig, tag: string = 'latest'): string {
  const filename = artifact.name.replace('{version}', config.version);
  return config.downloadBaseUrl
    .replace('{repository}', config.repository)
    .replace('{tag}', tag)
    .replace('{filename}', filename);
}

/**
 * Example: Download button component
 */
export function DownloadButton({
  platform,
  type,
  variant = 'primary'
}: {
  platform: keyof typeof config.artifacts;
  type: string;
  variant?: 'primary' | 'secondary';
}) {
  const artifact = config.artifacts[platform][type];

  if (!artifact) {
    console.error(`Artifact not found: ${platform}.${type}`);
    return null;
  }

  const downloadUrl = getDownloadUrl(artifact);

  return (
    <a
      href={downloadUrl}
      className={`download-btn download-btn--${variant}`}
      download
    >
      <div className="download-btn__icon">
        {/* Platform-specific icon */}
        {platform === 'windows' && '🪟'}
        {platform === 'linux' && '🐧'}
        {platform === 'macos' && '🍎'}
      </div>
      <div className="download-btn__content">
        <div className="download-btn__title">
          {artifact.displayName}
          {artifact.recommended && <span className="badge">Recommended</span>}
        </div>
        <div className="download-btn__meta">
          {artifact.platform} • {artifact.arch}
        </div>
      </div>
    </a>
  );
}

/**
 * Example: Full download section with all options
 */
export function DownloadSection() {
  return (
    <section className="download-section">
      <h2>Download Soul Player v{config.version}</h2>

      {/* Windows Downloads */}
      <div className="download-group">
        <h3>Windows</h3>
        <div className="download-options">
          <DownloadButton platform="windows" type="msi" variant="primary" />
          <DownloadButton platform="windows" type="nsis" variant="secondary" />
        </div>
      </div>

      {/* Linux Downloads */}
      <div className="download-group">
        <h3>Linux</h3>
        <div className="download-options">
          <DownloadButton platform="linux" type="deb" variant="primary" />
          <DownloadButton platform="linux" type="rpm" variant="secondary" />
          <DownloadButton platform="linux" type="appimage" variant="secondary" />
        </div>
      </div>

      {/* macOS Downloads */}
      <div className="download-group">
        <h3>macOS</h3>
        <div className="download-options">
          <DownloadButton platform="macos" type="dmg-arm" variant="primary" />
          <DownloadButton platform="macos" type="dmg-intel" variant="secondary" />
        </div>
      </div>
    </section>
  );
}

/**
 * Example: Get all download links as data (for SSG)
 */
export function getAllDownloadLinks() {
  const links: Array<{
    platform: string;
    type: string;
    displayName: string;
    url: string;
    recommended: boolean;
    arch: string;
  }> = [];

  for (const [platform, types] of Object.entries(config.artifacts)) {
    for (const [type, artifact] of Object.entries(types)) {
      links.push({
        platform,
        type,
        displayName: artifact.displayName,
        url: getDownloadUrl(artifact),
        recommended: artifact.recommended,
        arch: artifact.arch
      });
    }
  }

  return links;
}

/**
 * Example: Auto-detect user's platform and show recommended download
 */
export function SmartDownloadButton() {
  const userPlatform = detectPlatform();

  let recommendedArtifact: ArtifactConfig | null = null;
  let platform: keyof typeof config.artifacts | null = null;
  let type: string | null = null;

  // Find recommended artifact for user's platform
  for (const [p, types] of Object.entries(config.artifacts)) {
    for (const [t, artifact] of Object.entries(types)) {
      if (artifact.platform.toLowerCase() === userPlatform && artifact.recommended) {
        recommendedArtifact = artifact;
        platform = p as keyof typeof config.artifacts;
        type = t;
        break;
      }
    }
    if (recommendedArtifact) break;
  }

  if (!platform || !type) {
    return <div>Download links available below</div>;
  }

  return (
    <div className="smart-download">
      <DownloadButton platform={platform} type={type} variant="primary" />
      <p className="download-hint">
        Detected: {recommendedArtifact?.platform} ({recommendedArtifact?.arch})
      </p>
    </div>
  );
}

// Helper to detect user's platform
function detectPlatform(): string {
  if (typeof window === 'undefined') return 'unknown';

  const userAgent = window.navigator.userAgent.toLowerCase();
  const platform = window.navigator.platform.toLowerCase();

  if (platform.includes('win')) return 'windows';
  if (platform.includes('mac')) return 'macos';
  if (platform.includes('linux')) return 'linux';

  // Fallback to user agent
  if (userAgent.includes('windows')) return 'windows';
  if (userAgent.includes('mac')) return 'macos';
  if (userAgent.includes('linux')) return 'linux';

  return 'unknown';
}
