# Release Configuration Usage

This document explains how to use `.github/release-config.json` as the single source of truth for release artifacts across the project.

## Overview

The release config defines:
- **Artifact naming patterns** - Consistent file names across platforms
- **Download URLs** - Where users can download releases
- **Platform metadata** - Display names, architectures, recommendations

## Config Structure

```json
{
  "version": "0.0.1",
  "repository": "soulaudio/soul-player",
  "artifacts": {
    "windows": {
      "msi": {
        "name": "Soul Player_{version}_x64_en-US.msi",
        "artifactName": "windows-x86_64-pc-windows-msvc",
        "path": "msi",
        "displayName": "Windows Installer (MSI)",
        "platform": "Windows",
        "arch": "x64",
        "recommended": true
      }
    }
  }
}
```

## Usage in Installation Tests

Tests automatically:
1. **Try to download from GitHub Release** (real user experience)
2. **Fall back to workflow artifacts** if release isn't published yet

This ensures tests validate the exact files users will download.

### Example (Windows Test)

```yaml
- name: Download installer (Release or Artifact)
  run: |
    # Read config
    $config = Get-Content .github/release-config.json | ConvertFrom-Json
    $msiName = $config.artifacts.windows.msi.name -replace '{version}', $config.version

    # Try release first
    $downloadUrl = "https://github.com/$($config.repository)/releases/download/$TAG/$msiName"
    if (download succeeds) {
      Write-Host "✅ Downloaded from release"
    } else {
      Write-Host "⚠️ Falling back to artifacts"
      gh run download ...
    }
```

## Usage in Marketing Page

The marketing page can read this config to generate download links dynamically.

### Example: React Component

```typescript
import releaseConfig from '../../.github/release-config.json';

interface ReleaseConfig {
  version: string;
  repository: string;
  artifacts: {
    [platform: string]: {
      [type: string]: {
        name: string;
        displayName: string;
        platform: string;
        arch: string;
        recommended: boolean;
      };
    };
  };
}

const config = releaseConfig as ReleaseConfig;

function DownloadButton({ platform, type }: { platform: string; type: string }) {
  const artifact = config.artifacts[platform][type];
  const filename = artifact.name.replace('{version}', config.version);
  const downloadUrl = `https://github.com/${config.repository}/releases/latest/download/${filename}`;

  return (
    <a
      href={downloadUrl}
      className={artifact.recommended ? 'btn-primary' : 'btn-secondary'}
    >
      {artifact.displayName}
      {artifact.recommended && ' (Recommended)'}
    </a>
  );
}

// Usage
<DownloadButton platform="windows" type="msi" />
<DownloadButton platform="linux" type="deb" />
<DownloadButton platform="macos" type="dmg-arm" />
```

### Example: Static Site Generation

```javascript
// In a build script (e.g., Astro/Next.js)
import fs from 'fs';
import path from 'path';

const config = JSON.parse(
  fs.readFileSync('.github/release-config.json', 'utf-8')
);

export function getDownloadLinks() {
  const links = [];

  for (const [platform, types] of Object.entries(config.artifacts)) {
    for (const [type, artifact] of Object.entries(types)) {
      const filename = artifact.name.replace('{version}', config.version);
      const url = `https://github.com/${config.repository}/releases/latest/download/${filename}`;

      links.push({
        platform: artifact.platform,
        displayName: artifact.displayName,
        url,
        recommended: artifact.recommended,
        arch: artifact.arch
      });
    }
  }

  return links;
}
```

## Updating the Version

When bumping the version:

1. Update `version` in `.github/release-config.json`
2. Update `version` in `applications/desktop/src-tauri/tauri.conf.json`
3. Update `version` in root `package.json`
4. Create a git tag

Eventually, we should automate this with a version bump script.

## Benefits

✅ **Single Source of Truth** - All artifact names defined in one place
✅ **Consistency** - Marketing page and tests use identical URLs
✅ **Real User Testing** - Tests download from actual release when available
✅ **Easy Maintenance** - Change artifact names in one file
✅ **Type Safety** - Config can be validated with JSON schema

## Future Improvements

- [ ] Add JSON schema for validation
- [ ] Automate version bumping across all files
- [ ] Generate download page from config automatically
- [ ] Add checksums/signatures to config
- [ ] Support multiple versions/channels (stable, beta, nightly)
