# Soul Player Marketing Site

Marketing and documentation website for Soul Player, built with Next.js 15 and Nextra.

## Features

- **Landing Page**: Custom-designed hero, demo showcase, and feature highlights
- **Documentation**: Nextra-powered docs with search and navigation
- **Demo Mode**: Non-interactive component showcase using real UI from `@soul-player/shared`
- **Grainy Gradients**: Modern aesthetic with custom grain effects
- **SEO Optimized**: Metadata, OG tags, and static generation
- **Fly.io Ready**: Dockerized and configured for easy deployment

## Tech Stack

- **Framework**: Next.js 15 (App Router)
- **Documentation**: Nextra 4 with theme-docs
- **Styling**: Tailwind CSS
- **Animations**: Framer Motion
- **Icons**: Lucide React
- **Deployment**: Fly.io (Docker)

## Development

```bash
# Install dependencies (from workspace root)
yarn install

# Run development server
cd applications/marketing
yarn dev

# Open http://localhost:3001
```

## Project Structure

```
applications/marketing/
├── src/
│   ├── app/                    # Next.js App Router
│   │   ├── page.tsx           # Landing page
│   │   ├── layout.tsx         # Root layout
│   │   └── globals.css        # Global styles
│   ├── components/            # React components
│   │   ├── Hero.tsx           # Hero section
│   │   ├── DemoShowcase.tsx   # Demo player section
│   │   ├── DemoModeWrapper.tsx # Non-interactive wrapper
│   │   ├── GrainGradient.tsx  # Gradient component
│   │   └── Footer.tsx         # Footer
│   └── lib/                   # Utilities
├── pages/
│   └── docs/                  # Nextra documentation
│       ├── index.mdx          # Docs homepage
│       └── _meta.json         # Navigation config
├── public/                    # Static assets
├── Dockerfile                 # Multi-stage build
├── fly.toml                   # Fly.io config
└── theme.config.tsx           # Nextra theme

```

## Using Shared Components

The marketing site imports real UI components from `@soul-player/shared`:

```tsx
import { Button, Card } from '@soul-player/shared'
import { DemoModeWrapper } from '@/components/DemoModeWrapper'

// Wrap in DemoModeWrapper to make non-interactive
<DemoModeWrapper>
  <Button>Play</Button>
</DemoModeWrapper>
```

## Building

```bash
# Production build
yarn build

# Test production build locally
yarn start
```

## Deployment

### Fly.io (Recommended)

```bash
# Install Fly CLI
curl -L https://fly.io/install.sh | sh

# Login
fly auth login

# Deploy (first time)
fly launch

# Deploy updates
fly deploy

# Set custom domain
fly certs add player.soulaudio.co
```

### Environment Variables

Create `.env.local` for local development:

```bash
cp .env.example .env.local
```

For production, set via Fly.io:

```bash
fly secrets set NEXT_PUBLIC_BASE_URL=https://player.soulaudio.co
```

## Documentation

Add new documentation pages in `pages/docs/`:

```markdown
# pages/docs/getting-started.mdx

# Getting Started

Your content here...
```

Update navigation in `pages/docs/_meta.json`:

```json
{
  "index": "Introduction",
  "getting-started": "Getting Started"
}
```

## Customization

### Colors

Edit gradient colors in `src/components/GrainGradient.tsx`:

```tsx
<GrainGradient
  from="#your-color"
  via="#your-color"
  to="#your-color"
/>
```

### Branding

Update theme in `theme.config.tsx`:

```tsx
const config: DocsThemeConfig = {
  logo: <YourLogo />,
  primaryHue: 260, // Hue value (0-360)
  // ...
}
```

## Monitoring

```bash
# View logs
fly logs

# SSH into instance
fly ssh console

# Check status
fly status
```

## License

Part of the Soul Player project. See main repository for license.
