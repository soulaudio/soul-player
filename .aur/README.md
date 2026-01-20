# Soul Player AUR Package

This directory contains the PKGBUILD file for publishing Soul Player to the AUR (Arch User Repository).

## Package

### soul-player (Official Binary Package)
- Downloads pre-built binary from GitHub releases
- Official package maintained by Soul Player team
- Fast installation with pre-compiled binaries

## Publishing to AUR

### Prerequisites
1. Create an AUR account: https://aur.archlinux.org/register
2. Add your SSH key to your AUR account
3. Install AUR tools: `sudo pacman -S git base-devel`

### Initial Setup

```bash
# Clone AUR repository (first time only)
git clone ssh://aur@aur.archlinux.org/soul-player.git
cd soul-player

# Copy PKGBUILD
cp ../.aur/PKGBUILD PKGBUILD

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
cd soul-player

# Update PKGBUILD
# - Change pkgver=0.1.2
# - Change pkgrel=1 (reset to 1 for new version)

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
yay -S soul-player

# Using paru
paru -S soul-player

# Manual installation
git clone https://aur.archlinux.org/soul-player.git
cd soul-player
makepkg -si
```

## Automation

AUR publishing is **fully automated** via GitHub Actions. When a new release is published, the workflow automatically:

1. Downloads the binary from GitHub releases
2. Updates `PKGBUILD` with the new version
3. Generates `.SRCINFO` using Docker with Arch Linux
4. Commits and pushes to the AUR repository

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
- Updates the `soul-player` package on AUR
- Automatically handles `.SRCINFO` generation

## Links

- AUR Guidelines: https://wiki.archlinux.org/title/AUR_submission_guidelines
- PKGBUILD Reference: https://wiki.archlinux.org/title/PKGBUILD
- aurpublish tool: https://github.com/eli-schwartz/aurpublish
