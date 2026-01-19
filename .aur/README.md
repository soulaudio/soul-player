# Soul Player AUR Packages

This directory contains PKGBUILD files for publishing Soul Player to the AUR (Arch User Repository).

## Packages

### soul-player-bin (Binary Package)
- Downloads pre-built DEB package from GitHub releases
- Extracts and installs the binary
- **Recommended for most users** (faster installation)

### soul-player (Source Package)
- Builds from source code
- Requires Rust, Node.js, and build dependencies
- Takes longer but builds from scratch

## Publishing to AUR

### Prerequisites
1. Create an AUR account: https://aur.archlinux.org/register
2. Add your SSH key to your AUR account
3. Install AUR tools: `sudo pacman -S git base-devel`

### Initial Setup (Binary Package)

```bash
# Clone AUR repository (first time only)
git clone ssh://aur@aur.archlinux.org/soul-player-bin.git
cd soul-player-bin

# Copy PKGBUILD
cp ../.aur/PKGBUILD-bin PKGBUILD

# Update SHA256 sum
wget https://github.com/soulaudio/soul-player/releases/download/v0.1.1/soul_player_0.1.1_amd64.deb
sha256sum soul_player_0.1.1_amd64.deb
# Update sha256sums=('...') in PKGBUILD

# Generate .SRCINFO
makepkg --printsrcinfo > .SRCINFO

# Test build locally
makepkg -si

# Commit and push
git add PKGBUILD .SRCINFO
git commit -m "Initial release: 0.1.1"
git push origin master
```

### Updating Package (New Release)

```bash
cd soul-player-bin

# Update PKGBUILD
# - Change pkgver=0.1.2
# - Change pkgrel=1 (reset to 1 for new version)
# - Update sha256sum

# Regenerate .SRCINFO
makepkg --printsrcinfo > .SRCINFO

# Test build
makepkg -si

# Commit and push
git add PKGBUILD .SRCINFO
git commit -m "Update to 0.1.2"
git push
```

## User Installation

After publishing to AUR, users can install with:

```bash
# Using yay (most common AUR helper)
yay -S soul-player-bin

# Using paru
paru -S soul-player-bin

# Manual installation
git clone https://aur.archlinux.org/soul-player-bin.git
cd soul-player-bin
makepkg -si
```

## Automation

AUR publishing is **fully automated** via GitHub Actions. When a new release is published, the workflow automatically:

1. Downloads the DEB package from GitHub releases
2. Calculates the SHA256 checksum
3. Updates `PKGBUILD-bin` with the new version and checksum
4. Generates `.SRCINFO` using Docker with Arch Linux
5. Commits and pushes to the AUR repository

### Required GitHub Secrets

To enable automated publishing, add these secrets to your GitHub repository:

1. **`AUR_USERNAME`**: Your AUR username
2. **`AUR_EMAIL`**: Your AUR email address
3. **`AUR_SSH_PRIVATE_KEY`**: Your SSH private key with AUR access

To set up secrets:
1. Go to `https://github.com/soulaudio/soul-player/settings/secrets/actions`
2. Click "New repository secret"
3. Add each secret above

### SSH Key Setup for AUR

```bash
# Generate SSH key for AUR (if you don't have one)
ssh-keygen -t ed25519 -C "your_email@example.com" -f ~/.ssh/aur

# Add public key to AUR account
cat ~/.ssh/aur.pub
# Go to https://aur.archlinux.org/account/ and add the public key

# Add private key to GitHub Secrets
cat ~/.ssh/aur
# Copy the entire output and add as AUR_SSH_PRIVATE_KEY secret
```

### How It Works

The `publish-aur` job in `.github/workflows/release.yml`:
- Runs after the release is published
- Uses `KSXGitHub/github-actions-deploy-aur@v4` action
- Updates the `soul-player-bin` package on AUR
- Automatically handles `.SRCINFO` generation

### Testing Updates Locally

You can manually test the update script:

```bash
# Calculate SHA256 of a DEB file
SHA256=$(sha256sum path/to/soul_player_0.1.2_amd64.deb | awk '{print $1}')

# Update PKGBUILD-bin
./.aur/update-pkgbuild.sh 0.1.2 "$SHA256"

# Verify changes
cat .aur/PKGBUILD-bin
```

## Links

- AUR Guidelines: https://wiki.archlinux.org/title/AUR_submission_guidelines
- PKGBUILD Reference: https://wiki.archlinux.org/title/PKGBUILD
- aurpublish tool: https://github.com/eli-schwartz/aurpublish
