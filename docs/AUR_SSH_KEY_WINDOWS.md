# AUR SSH Key Setup - Windows Instructions

## Method 1: PowerShell (Recommended)

**Step 1: Open PowerShell**
- Press `Win + X` and select "Windows PowerShell" or "Terminal"

**Step 2: Generate the SSH key**
```powershell
# Create .ssh directory if it doesn't exist
New-Item -ItemType Directory -Force -Path "$env:USERPROFILE\.ssh"

# Generate ed25519 key in PEM format
ssh-keygen -t ed25519 -C "sebastian.stupak@pm.me" -f "$env:USERPROFILE\.ssh\aur_new" -m PEM

# When prompted for passphrase, just press Enter twice (leave empty)
```

**Step 3: View the private key (for GitHub secret)**
```powershell
Get-Content "$env:USERPROFILE\.ssh\aur_new"
```

**Step 4: View the public key (for AUR account)**
```powershell
Get-Content "$env:USERPROFILE\.ssh\aur_new.pub"
```

**Step 5: Copy the keys**
```powershell
# Copy private key to clipboard (for GitHub secret)
Get-Content "$env:USERPROFILE\.ssh\aur_new" | Set-Clipboard
Write-Host "Private key copied to clipboard - paste this into GitHub secret AUR_SSH_PRIVATE_KEY"
pause

# Copy public key to clipboard (for AUR account)
Get-Content "$env:USERPROFILE\.ssh\aur_new.pub" | Set-Clipboard
Write-Host "Public key copied to clipboard - paste this into AUR account at https://aur.archlinux.org/account/"
```

---

## Method 2: Git Bash (Alternative)

If PowerShell doesn't work, use Git Bash (comes with Git for Windows):

**Step 1: Open Git Bash**
- Right-click in any folder and select "Git Bash Here"

**Step 2: Generate the key**
```bash
# Generate key in PEM format
ssh-keygen -t ed25519 -C "sebastian.stupak@pm.me" -f ~/.ssh/aur_new -m PEM -N ""
```

**Step 3: View the keys**
```bash
# View private key
cat ~/.ssh/aur_new

# View public key
cat ~/.ssh/aur_new.pub
```

---

## Method 3: Manual Steps (If ssh-keygen fails)

If ssh-keygen doesn't work or isn't installed:

### Install OpenSSH Client (Windows 10/11)

1. Press `Win + I` to open Settings
2. Go to **Apps** > **Optional Features**
3. Click **Add a feature**
4. Search for "OpenSSH Client"
5. Click **Install**
6. Restart PowerShell and try Method 1 again

---

## What to Do Next

### 1. Update GitHub Secret

1. Go to: https://github.com/soulaudio/soul-player/settings/secrets/actions
2. Click on `AUR_SSH_PRIVATE_KEY` (or "New repository secret" if it doesn't exist)
3. Paste the **entire private key** including the header and footer:
   ```
   -----BEGIN OPENSSH PRIVATE KEY-----
   [key content]
   -----END OPENSSH PRIVATE KEY-----
   ```
4. Click **Update secret** or **Add secret**

### 2. Add Public Key to AUR Account

1. Go to: https://aur.archlinux.org/account/
2. Login with your AUR credentials
3. Scroll to **SSH Public Key**
4. Paste the **public key** (starts with `ssh-ed25519 AAAA...`)
5. Click **Update**

### 3. Verify the GitHub Secrets

Make sure these three secrets exist:
- ✅ `AUR_SSH_PRIVATE_KEY` - The entire private key
- ✅ `AUR_USERNAME` - Your AUR username
- ✅ `AUR_EMAIL` - sebastian.stupak@pm.me

---

## Test the Key (Optional)

From PowerShell or Git Bash:

```powershell
# Test SSH connection to AUR
ssh -i "$env:USERPROFILE\.ssh\aur_new" aur@aur.archlinux.org
```

Or in Git Bash:
```bash
ssh -i ~/.ssh/aur_new aur@aur.archlinux.org
```

**Expected output:**
```
Hi <username>! You've successfully authenticated, but I do not provide shell access.
Connection to aur.archlinux.org closed.
```

---

## Troubleshooting

### Error: "ssh-keygen is not recognized"

**Solution:** Install OpenSSH Client (see Method 3 above)

### Error: "Saving key failed: permission denied"

**Solution:** Run PowerShell as Administrator:
- Press `Win + X`
- Select "Windows PowerShell (Admin)" or "Terminal (Admin)"

### Key format issues

If the key doesn't work, make sure it's in PEM format:
- The private key should start with `-----BEGIN OPENSSH PRIVATE KEY-----`
- Include the ENTIRE key including the first and last lines

### Still not working?

If automated publishing continues to fail, you can manually publish to AUR:

1. Install WSL: `wsl --install` (in PowerShell as Admin)
2. Follow the manual publishing steps in the main guide
3. Or wait until you have access to a Linux/macOS machine

---

## Quick Copy-Paste Commands

Run these in PowerShell one by one:

```powershell
# 1. Generate key
ssh-keygen -t ed25519 -C "sebastian.stupak@pm.me" -f "$env:USERPROFILE\.ssh\aur_new" -m PEM

# 2. Copy private key
Get-Content "$env:USERPROFILE\.ssh\aur_new" | Set-Clipboard
Write-Host "`n✅ Private key copied! Go to GitHub secrets and paste."
Write-Host "URL: https://github.com/soulaudio/soul-player/settings/secrets/actions`n"
pause

# 3. Copy public key
Get-Content "$env:USERPROFILE\.ssh\aur_new.pub" | Set-Clipboard
Write-Host "`n✅ Public key copied! Go to AUR account and paste."
Write-Host "URL: https://aur.archlinux.org/account/`n"
```
