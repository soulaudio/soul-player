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

Consider automating AUR updates in your release workflow:

1. Generate SHA256 sums automatically
2. Update PKGBUILD version
3. Push to AUR repository
4. Use `aurpublish` or similar tools

## Links

- AUR Guidelines: https://wiki.archlinux.org/title/AUR_submission_guidelines
- PKGBUILD Reference: https://wiki.archlinux.org/title/PKGBUILD
- aurpublish tool: https://github.com/eli-schwartz/aurpublish
