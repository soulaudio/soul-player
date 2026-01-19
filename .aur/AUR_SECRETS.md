# AUR Publishing - Required GitHub Secrets

This document describes the GitHub Secrets required for automated AUR (Arch User Repository) publishing.

## Required Secrets

The following secrets must be added to the GitHub repository for AUR automation to work:

### 1. `AUR_USERNAME`
- **Description**: Your AUR account username
- **Example**: `john-doe`
- **How to get**: Your username from https://aur.archlinux.org/account/

### 2. `AUR_EMAIL`
- **Description**: Email address associated with your AUR account
- **Example**: `john.doe@example.com`
- **How to get**: Your email from https://aur.archlinux.org/account/

### 3. `AUR_SSH_PRIVATE_KEY`
- **Description**: SSH private key with access to push to AUR
- **Format**: Full private key including `-----BEGIN` and `-----END` lines
- **How to generate**: See instructions below

## Setting Up SSH Keys for AUR

### Step 1: Generate SSH Key Pair

```bash
# Generate a new Ed25519 SSH key (recommended)
ssh-keygen -t ed25519 -C "your_email@example.com" -f ~/.ssh/aur

# Or use RSA if Ed25519 is not supported
ssh-keygen -t rsa -b 4096 -C "your_email@example.com" -f ~/.ssh/aur
```

When prompted:
- Enter a passphrase (optional, but not recommended for CI use)
- For CI/CD, leave the passphrase empty (just press Enter)

This will create two files:
- `~/.ssh/aur` - Private key (keep secret!)
- `~/.ssh/aur.pub` - Public key (add to AUR)

### Step 2: Add Public Key to AUR

```bash
# Display your public key
cat ~/.ssh/aur.pub
```

Copy the output and:
1. Go to https://aur.archlinux.org/account/
2. Click "My Account"
3. Paste the public key in the "SSH Public Key" field
4. Save

### Step 3: Test SSH Access

```bash
# Test connection to AUR
ssh -T aur@aur.archlinux.org
```

You should see: `Hi <username>! You've successfully authenticated, but I do not provide shell access.`

### Step 4: Add Private Key to GitHub Secrets

```bash
# Display your private key
cat ~/.ssh/aur
```

Copy the **entire output** including the `-----BEGIN` and `-----END` lines.

Then:
1. Go to your GitHub repository
2. Navigate to Settings > Secrets and variables > Actions
3. Click "New repository secret"
4. Name: `AUR_SSH_PRIVATE_KEY`
5. Value: Paste the entire private key
6. Click "Add secret"

Repeat for `AUR_USERNAME` and `AUR_EMAIL`.

## Security Considerations

- **Never commit** your private key to the repository
- Use separate SSH keys for different services
- Store private keys securely
- Consider using SSH agent forwarding for local development
- Rotate keys periodically

## Verifying Setup

After adding secrets, the next release will automatically trigger the AUR publishing workflow.

Check the workflow status:
1. Go to Actions tab in GitHub
2. Select the release workflow
3. Look for the "Publish to AUR" job
4. Verify it completes successfully

If the job fails:
- Check that all three secrets are set correctly
- Verify SSH key is added to AUR account
- Check AUR repository permissions

## Troubleshooting

### Authentication Failed
- Verify SSH public key is added to AUR account
- Ensure private key in GitHub Secrets matches the public key
- Check that the key has no passphrase (CI can't handle interactive passphrases)

### Permission Denied
- Verify you own the `soul-player-bin` package on AUR
- Check that your AUR account has push permissions
- Ensure the SSH key is associated with the correct AUR account

### Invalid Key Format
- Ensure the entire private key is copied (including BEGIN/END lines)
- Check for no extra whitespace or line breaks
- Verify the key format matches what `ssh-keygen` generated

## Manual Publishing (Fallback)

If automated publishing fails, you can manually publish:

```bash
# Clone AUR repository
git clone ssh://aur@aur.archlinux.org/soul-player-bin.git
cd soul-player-bin

# Update files
cp ../.aur/PKGBUILD-bin PKGBUILD
makepkg --printsrcinfo > .SRCINFO

# Commit and push
git add PKGBUILD .SRCINFO
git commit -m "Update to X.Y.Z"
git push origin master
```

## References

- [AUR Submission Guidelines](https://wiki.archlinux.org/title/AUR_submission_guidelines)
- [SSH Key Setup](https://wiki.archlinux.org/title/SSH_keys)
- [GitHub Actions Secrets](https://docs.github.com/en/actions/security-guides/encrypted-secrets)
- [KSXGitHub/github-actions-deploy-aur](https://github.com/KSXGitHub/github-actions-deploy-aur)
