# Features Showcase Components

This directory contains specialized 3D showcase components for demonstrating Soul Player's features on the marketing site.

## Components

### MobileShowcase

A 3D interactive showcase component that displays Soul Player's mobile and portable device ecosystem with realistic device frames and scroll-triggered animations.

**Features:**
- 3D device frames (Phone, Tablet, DAP) with realistic bezels and shadows
- Scroll-based parallax rotation and scaling effects
- Animated sync indicators between devices
- Responsive mobile UI mockups at scale
- CSS-only device frames (no image dependencies)
- Smooth scroll-triggered animations

**Usage:**
```tsx
import { MobileShowcase } from '@/components/features/MobileShowcase'

export function MyPage() {
  return (
    <div>
      <MobileShowcase />
    </div>
  )
}
```

**Integration with WhySoulPlayer:**
See `MobileShowcase.example.tsx` for examples of how to integrate into the existing marketing page layout.

**Visual Design:**
- Device frames use CSS borders and shadows for realistic depth
- Screen reflections via gradient overlays
- Scroll progress controls device rotation (-30° to +30°)
- Z-index layering creates depth perception
- Sync indicator appears during scroll middle range (30-70%)

**Performance:**
- Uses `transform3d` and `will-change` for GPU acceleration
- Passive scroll listeners for smooth performance
- No heavy dependencies - pure CSS + React

**Customization:**
- Adjust device dimensions in `DeviceFrame` component
- Modify scroll animation curves in transform calculations
- Change device positions via translate values
- Customize UI mockup in `MobileScreen` component

---

### DiscoveryShowcase

A 3D showcase component demonstrating metadata integration from external music services (Discogs, Bandcamp, MusicBrainz, AcoustID).

**Features:**
- 3D rotation effect based on scroll position
- Animated data flow lines from services to metadata
- Before/after metadata comparison (auto-toggles every 4s)
- Service badges that cycle automatically (3s intervals)
- Floating particle effects during enhanced mode
- Feature highlights explaining discovery benefits

**Usage:**
```tsx
import { DiscoveryShowcase } from '@/components/features/DiscoveryShowcase'

export function MyPage() {
  return (
    <div>
      <DiscoveryShowcase />
    </div>
  )
}
```

**Visual Structure:**
```
┌─────────────────────────────────────────────┐
│  Header: "Actually Discover Music"         │
├──────────────┬─────────┬──────────────────┤
│  Services    │  Hub    │  Album Card      │
│  (4 badges)  │  Icon   │  Before/After    │
│  + Data flow │ (rotate)│  Metadata        │
├──────────────┴─────────┴──────────────────┤
│  Feature Highlights (3 cards)             │
└───────────────────────────────────────────┘
```

**Auto-Cycling:**
- Service badges cycle every 3 seconds
- Before/after metadata toggles every 4 seconds
- Data flow animation on active service

**Service Colors:**
- Discogs: Purple to Pink gradient
- Bandcamp: Cyan to Blue gradient
- MusicBrainz: Orange to Red gradient
- AcoustID: Green to Emerald gradient

**Dependencies:**
- `framer-motion` - Scroll-based 3D transforms
- `lucide-react` - Icons (Music, Database, Sparkles, ExternalLink)
- `react-awesome-reveal` - FadeIn animation wrapper

---

## Design Principles

All showcase components follow these principles:

1. **Apple-style aesthetics** - Clean, premium, high-quality visuals
2. **Scroll-triggered animations** - Engage users as they explore the page
3. **3D perspective** - Use CSS transforms for depth and realism
4. **Performance-first** - GPU-accelerated animations, passive listeners
5. **Theme-aware** - All colors use CSS variables from the theme system
6. **Accessibility** - Semantic HTML, proper ARIA labels where needed

## File Structure

```
features/
├── README.md                      # This file
├── MobileShowcase.tsx            # 3D mobile device showcase
├── MobileShowcase.example.tsx    # Integration examples
├── AudiophileShowcase.tsx        # Audio quality showcase
├── DiscoveryShowcase.tsx         # Metadata integration showcase
└── DiscoveryShowcase.example.tsx # Discovery usage examples
```

## Related Files

- `applications/marketing/src/components/animations/FadeIn.tsx` - Fade animation wrapper
- `applications/marketing/src/components/WhySoulPlayer.tsx` - Main features page
- `applications/shared/src/styles/globals.css` - Theme CSS variables

## Contributing

When creating new showcase components:

1. Follow the naming pattern: `[Feature]Showcase.tsx`
2. Include scroll-based interactivity
3. Use theme CSS variables for all colors
4. Export from `components/index.ts`
5. Create a `.example.tsx` file showing integration
6. Update this README with component details
