# Marketing Site Setup

## Prerequisites

This project requires:
- **Node.js**: v20.10.0 or higher
- **Yarn**: 4.0.2 (managed via Corepack)
- **Corepack**: Enabled for package manager management

## Initial Setup

### 1. Upgrade Node.js

**Using nvm (recommended):**
```bash
# Install nvm if not already installed
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash

# Install and use Node 20+
nvm install 20
nvm use 20
```

**Or download from:** https://nodejs.org/

### 2. Enable Corepack

```bash
corepack enable
```

This enables Yarn 4.0.2 as specified in `package.json`.

### 3. Install Dependencies

From the **workspace root** (the soul-player directory):

```bash
yarn install
```

This will install dependencies for all workspace packages including the marketing site.

### 4. Run Development Server

```bash
# From workspace root
yarn workspace @soul-player/marketing dev

# Or from applications/marketing/
cd applications/marketing
yarn dev
```

Visit http://localhost:3001

## Project Structure

```
applications/marketing/
├── src/
│   ├── app/                    # Next.js 15 App Router
│   │   ├── page.tsx           # Landing page
│   │   ├── layout.tsx         # Root layout
│   │   └── globals.css        # Global styles + grain effect
│   ├── components/
│   │   ├── Hero.tsx           # Hero section with gradient
│   │   ├── DemoShowcase.tsx   # Demo player section
│   │   ├── DemoModeWrapper.tsx # Non-interactive wrapper
│   │   ├── GrainGradient.tsx  # Grainy gradient component
│   │   └── Footer.tsx         # Footer component
│   └── lib/                   # Utilities (future)
├── pages/
│   └── docs/                  # Nextra documentation
│       ├── index.mdx          # Docs homepage
│       └── _meta.json         # Navigation config
├── public/                    # Static assets
├── Dockerfile                 # Multi-stage production build
├── fly.toml                   # Fly.io deployment config
├── theme.config.tsx           # Nextra theme config
└── next.config.mjs            # Next.js configuration
```

## Development

### Run locally

```bash
yarn dev
```

### Type checking

```bash
yarn type-check
```

### Linting

```bash
yarn lint
```

### Build for production

```bash
yarn build
```

### Test production build

```bash
yarn build && yarn start
```

## Using Shared Components

The marketing site imports from `@soul-player/shared`:

```tsx
import { Button } from '@soul-player/shared'
import { DemoModeWrapper } from '@/components/DemoModeWrapper'

export function MySection() {
  return (
    <DemoModeWrapper>
      <Button>This button is non-interactive</Button>
    </DemoModeWrapper>
  )
}
```

The `DemoModeWrapper` makes any component non-interactive while preserving its visual appearance.

## Customization

### Update Colors

Edit `src/components/GrainGradient.tsx`:

```tsx
<GrainGradient
  from="#your-color"
  via="#your-color"
  to="#your-color"
>
  {children}
</GrainGradient>
```

### Update Branding

Edit `theme.config.tsx` for documentation site branding.

### Add Documentation Pages

Create `.mdx` files in `pages/docs/`:

```markdown
# pages/docs/getting-started.mdx

# Getting Started

Your content...
```

Update `pages/docs/_meta.json`:

```json
{
  "index": "Introduction",
  "getting-started": "Getting Started"
}
```

## Deployment

See [DEPLOYMENT.md](./DEPLOYMENT.md) for complete Fly.io deployment instructions.

Quick deploy:

```bash
# First time
fly launch

# Subsequent deploys
fly deploy
```

## Troubleshooting

### Yarn version error

```
error This project's package.json defines "packageManager": "yarn@4.0.2"
```

**Solution**: Enable Corepack:
```bash
corepack enable
```

### Node version too old

```
npm WARN EBADENGINE required: { node: '^20.10.0 || ^22.11.0 || >=24.0.0' }
```

**Solution**: Upgrade Node.js to v20+

### Shared package not found

```
Error: Cannot find module '@soul-player/shared'
```

**Solution**: Install from workspace root:
```bash
cd ../.. # Go to workspace root
yarn install
```

### Port already in use

The marketing site runs on port 3001 to avoid conflicts with the desktop app.

To change the port, edit `package.json`:
```json
"scripts": {
  "dev": "next dev -p 3002"
}
```

## Moon Tasks

If using Moon task runner:

```bash
# Run dev server
moon run marketing:dev

# Build
moon run marketing:build

# Type check
moon run marketing:type-check

# Deploy
moon run marketing:deploy
```

## Environment Variables

Create `.env.local` for local development:

```bash
cp .env.example .env.local
```

Edit values as needed:
```env
NEXT_PUBLIC_BASE_URL=http://localhost:3001
```

For production, set via Fly.io:
```bash
fly secrets set NEXT_PUBLIC_BASE_URL=https://player.soulaudio.co
```

## Next Steps

1. **Add real player components** from `@soul-player/shared` to `DemoShowcase.tsx`
2. **Create content** for additional landing page sections
3. **Add screenshots/videos** to `public/` directory
4. **Write documentation** in `pages/docs/`
5. **Set up deployment** following DEPLOYMENT.md
6. **Configure custom domain** at Fly.io

## Resources

- [Next.js 15 Docs](https://nextjs.org/docs)
- [Nextra Docs](https://nextra.site)
- [Tailwind CSS](https://tailwindcss.com)
- [Fly.io Docs](https://fly.io/docs)
