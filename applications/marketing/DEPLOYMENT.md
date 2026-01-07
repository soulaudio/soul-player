# Deployment Guide

Step-by-step guide to deploy the Soul Player marketing site to Fly.io.

## Prerequisites

1. **Fly.io Account**: Sign up at [fly.io](https://fly.io)
2. **Fly CLI**: Install the CLI tool
   ```bash
   curl -L https://fly.io/install.sh | sh
   ```
3. **Domain**: `player.soulaudio.co` configured

## First-Time Setup

### 1. Login to Fly.io

```bash
fly auth login
```

### 2. Launch the App

From the `applications/marketing` directory:

```bash
fly launch
```

You'll be prompted with:
- **App name**: Use `soul-player-marketing` or your preferred name
- **Region**: Choose closest to your users (e.g., `iad` for US East)
- **Database**: Skip (not needed for static site)
- **Deploy now**: Say "No" (we'll deploy manually)

This creates a `fly.toml` file (already included).

### 3. Configure Secrets (Optional)

If you need environment variables:

```bash
fly secrets set NEXT_PUBLIC_BASE_URL=https://player.soulaudio.co
```

### 4. Deploy

```bash
fly deploy
```

This will:
1. Build the Docker image
2. Push to Fly.io registry
3. Deploy to your app
4. Automatically start the app

### 5. Configure Custom Domain

```bash
# Add your custom domain
fly certs add player.soulaudio.co

# Get DNS instructions
fly certs show player.soulaudio.co
```

Add the DNS records shown to your domain registrar:
- **A record**: Point to Fly.io IP
- **AAAA record**: Point to Fly.io IPv6

### 6. Verify Deployment

```bash
# Check status
fly status

# View logs
fly logs

# Open in browser
fly open
```

## Subsequent Deployments

After the first deployment, updates are simple:

```bash
# From applications/marketing/
fly deploy
```

## Deployment Checklist

Before deploying:

- [ ] Run type-check: `yarn type-check`
- [ ] Run lint: `yarn lint`
- [ ] Test build locally: `yarn build`
- [ ] Update environment variables if needed
- [ ] Test Docker build: `docker build -t soul-player-marketing .`

## Monitoring

### View Logs

```bash
# Real-time logs
fly logs

# Last 100 lines
fly logs --lines 100
```

### SSH Into Instance

```bash
fly ssh console
```

### Check Resource Usage

```bash
fly status
fly vm status
```

## Scaling

### Adjust Resources

Edit `fly.toml`:

```toml
[[vm]]
  memory = "1024mb"  # Increase from 512mb
  cpu_kind = "shared"
  cpus = 2          # Increase from 1
```

Then deploy:

```bash
fly deploy
```

### Auto-Scaling

The current configuration uses auto-stop/start:

```toml
[http_service]
  auto_stop_machines = "stop"
  auto_start_machines = true
  min_machines_running = 0  # Scale to zero when idle
```

To keep machines always running:

```toml
min_machines_running = 1
```

## Cost Optimization

Current configuration costs approximately:
- **Compute**: $0-5/month (scales to zero)
- **Bandwidth**: Pay as you go

Tips:
- Keep `min_machines_running = 0` for low-traffic sites
- Use CDN for static assets (future optimization)

## Rollback

If a deployment fails:

```bash
# List releases
fly releases

# Rollback to previous version
fly releases rollback
```

## Environment-Specific Deployments

### Staging Environment

Create a staging app:

```bash
fly apps create soul-player-marketing-staging
fly deploy --app soul-player-marketing-staging
```

### Production Best Practices

1. **Use GitHub Actions** for CI/CD:

```yaml
# .github/workflows/deploy.yml
name: Deploy to Fly.io

on:
  push:
    branches: [main]
    paths:
      - 'applications/marketing/**'

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: superfly/flyctl-actions/setup-flyctl@master
      - run: flyctl deploy --remote-only
        working-directory: applications/marketing
        env:
          FLY_API_TOKEN: ${{ secrets.FLY_API_TOKEN }}
```

2. **Set up monitoring**: Use Fly.io metrics or external tools

3. **Configure health checks**: Already included in `fly.toml`

## Troubleshooting

### Build Fails

```bash
# Check Docker build locally
docker build -t soul-player-marketing .
docker run -p 3000:3000 soul-player-marketing
```

### App Won't Start

```bash
# Check logs
fly logs

# Common issues:
# - Missing environment variables
# - Port mismatch (ensure PORT=3000)
# - Node version incompatibility
```

### DNS Issues

```bash
# Verify certificate status
fly certs show player.soulaudio.co

# May take up to 24 hours for DNS to propagate
```

### High Memory Usage

```bash
# Increase VM memory in fly.toml
[[vm]]
  memory = "1024mb"
```

## Support

- **Fly.io Docs**: https://fly.io/docs
- **Community**: https://community.fly.io
- **Status**: https://status.fly.io

## Quick Commands Reference

```bash
# Deploy
fly deploy

# Logs
fly logs

# Status
fly status

# Scale
fly scale count 2

# SSH
fly ssh console

# Restart
fly apps restart

# Open in browser
fly open

# List apps
fly apps list

# Destroy app (careful!)
fly apps destroy soul-player-marketing
```
