# AUR SSH Key Fix

## Problem
The AUR publishing workflow fails with:
```
Load key "/home/builder/.ssh/aur": error in libcrypto
```

This means the SSH private key stored in `AUR_SSH_PRIVATE_KEY` secret is in an incompatible format.

## Solution

### Option 1: Convert Existing Key (If You Have It)

If you have access to your current AUR SSH key:

```bash
# Convert OpenSSH format to PEM format
ssh-keygen -p -m PEM -f ~/.ssh/aur

# This will prompt you to enter the passphrase (if any)
# Then it will convert the key to PEM format in-place

# View the converted key
cat ~/.ssh/aur
```

Copy the entire output (including `-----BEGIN RSA PRIVATE KEY-----` and `-----END RSA PRIVATE KEY-----`) and update the GitHub secret.

### Option 2: Generate New Key Pair

If you don't have access to the original key or want to start fresh:

```bash
# Generate a new ed25519 key in PEM format
ssh-keygen -t ed25519 -C "aur@soulaudio.co" -f ~/.ssh/aur_new -m PEM

# Or use RSA if ed25519 isn't supported
ssh-keygen -t rsa -b 4096 -C "aur@soulaudio.co" -f ~/.ssh/aur_new -m PEM

# IMPORTANT: Press Enter when asked for a passphrase (leave it empty)

# View the private key
cat ~/.ssh/aur_new

# View the public key
cat ~/.ssh/aur_new.pub
```

### Option 3: Convert Using OpenSSL (Alternative Method)

If ssh-keygen doesn't work, use OpenSSL:

```bash
# For RSA keys
ssh-keygen -f ~/.ssh/aur -p -m PEM

# Or convert existing key
openssl rsa -in ~/.ssh/aur -out ~/.ssh/aur_pem -traditional

# View the converted key
cat ~/.ssh/aur_pem
```

## Update GitHub Secrets

1. Go to: https://github.com/soulaudio/soul-player/settings/secrets/actions

2. **Update or add these secrets:**
   - `AUR_SSH_PRIVATE_KEY`: The entire contents of the private key (PEM format)
     ```
     -----BEGIN RSA PRIVATE KEY-----
     [key content]
     -----END RSA PRIVATE KEY-----
     ```
   - `AUR_USERNAME`: Your AUR username
   - `AUR_EMAIL`: Your AUR email address

3. **Add public key to AUR account:**
   - Go to: https://aur.archlinux.org/account/
   - Add the public key (`~/.ssh/aur.pub` or `~/.ssh/aur_new.pub`)

## Verify Key Format

A correctly formatted PEM key should look like:

```
-----BEGIN RSA PRIVATE KEY-----
MIIEpAIBAAKCAQEA...
[multiple lines of base64]
...ending with base64=
-----END RSA PRIVATE KEY-----
```

Or for ed25519:

```
-----BEGIN OPENSSH PRIVATE KEY-----
b3BlbnNzaC1rZXktdjEAAAAACmFlczI1Ni1jdHIAAAAGYmNyeXB0AAAAGAAAABDu...
[multiple lines of base64]
-----END OPENSSH PRIVATE KEY-----
```

## Test Locally (Optional)

Before updating GitHub secrets, test the key works:

```bash
# Test SSH connection to AUR
ssh -i ~/.ssh/aur_new aur@aur.archlinux.org

# You should see:
# Hi <username>! You've successfully authenticated...
# Connection to aur.archlinux.org closed.
```

## After Updating Secrets

1. Go to: https://github.com/soulaudio/soul-player/actions
2. Find the failed "Publish to AUR" job
3. Re-run just the AUR publishing job, or
4. Manually trigger a new release workflow

## Manual AUR Publishing (Fallback)

If automated publishing still fails, you can manually publish:

```bash
# Clone AUR repository
git clone ssh://aur@aur.archlinux.org/soul-player.git
cd soul-player

# Copy updated PKGBUILD
cp /path/to/soul-player/.aur/PKGBUILD .

# Update version in PKGBUILD
# Edit PKGBUILD and change:
# pkgver=0.1.3
# pkgrel=1

# Generate .SRCINFO
makepkg --printsrcinfo > .SRCINFO

# Commit and push
git add PKGBUILD .SRCINFO
git commit -m "Update to 0.1.3"
git push
```
