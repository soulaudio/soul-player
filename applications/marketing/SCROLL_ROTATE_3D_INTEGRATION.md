# ScrollRotate3D Integration Guide

This guide shows how to integrate the new `ScrollRotate3D` component into the existing marketing site.

## Quick Start

The component is ready to use! Import and wrap any element you want to animate:

```tsx
import { ScrollRotate3D } from '@/components/animations'

<ScrollRotate3D>
  <YourContent />
</ScrollRotate3D>
```

## Integration with WhySoulPlayer.tsx

Here's how to add 3D scroll rotation to the feature sections in `WhySoulPlayer.tsx`:

### Option 1: Wrap PlaceholderImage Components

Add rotation to the feature images:

```tsx
// In the FeatureSection component, update the image variable:

const image = (
  <FadeIn direction={imageFirst ? 'left' : 'right'} delay={0.2}>
    <ScrollRotate3D
      initialRotateY={imageFirst ? -12 : 12}
      maxRotateY={imageFirst ? 12 : -12}
      initialRotateX={-4}
      maxRotateX={4}
    >
      <PlaceholderImage type={imageType} />
    </ScrollRotate3D>
  </FadeIn>
)
```

This will make feature images rotate as users scroll, with the direction depending on whether the image is on the left or right.

### Option 2: Wrap SupportSection Card

Add 3D scroll rotation to the support section card:

```tsx
// In SupportSection component, wrap the card div:

<ScrollRotate3D
  initialRotateY={10}
  maxRotateY={-10}
  initialRotateX={-3}
  maxRotateX={3}
  minScale={0.96}
  maxScale={1.02}
>
  <div
    ref={cardRef}
    style={{
      transformStyle: 'preserve-3d',
      willChange: 'transform',
    }}
    onMouseMove={handleMouseMove}
    onMouseLeave={handleMouseLeave}
  >
    {/* Card content */}
  </div>
</ScrollRotate3D>
```

**Note:** The support section already has mouse-based 3D rotation. You could:
1. Remove mouse rotation and use scroll rotation only
2. Combine both effects (scroll + mouse)
3. Keep them separate

### Option 3: Selective Application

Apply to specific sections only:

```tsx
// Only add to the "Actually YOUR Music" section
<div className="py-16 md:py-24 lg:py-32">
  <div className="max-w-7xl mx-auto px-6 md:px-8 lg:px-12">
    <div className="grid grid-cols-1 lg:grid-cols-2 gap-12 md:gap-16 lg:gap-24 items-center">
      <div>{content}</div>
      <ScrollRotate3D>
        <FadeIn direction="right" delay={0.2}>
          <PlaceholderImage type="library" />
        </FadeIn>
      </ScrollRotate3D>
    </div>
  </div>
</div>
```

## Recommended Settings by Use Case

### Feature Section Images (Left/Right Layout)

For images alternating left/right:

```tsx
// Right-aligned images
<ScrollRotate3D
  initialRotateY={12}
  maxRotateY={-12}
  initialRotateX={-4}
  maxRotateX={4}
  minScale={0.96}
  maxScale={1}
>

// Left-aligned images (mirror the rotation)
<ScrollRotate3D
  initialRotateY={-12}
  maxRotateY={12}
  initialRotateX={-4}
  maxRotateX={4}
  minScale={0.96}
  maxScale={1}
>
```

### Hero Screenshots/Large Visuals

For prominent product screenshots:

```tsx
<ScrollRotate3D
  initialRotateY={15}
  maxRotateY={-15}
  initialRotateX={-5}
  maxRotateX={5}
  minScale={0.95}
  maxScale={1.02}
  perspective={1200}
>
```

### Call-to-Action Cards

For support/CTA sections:

```tsx
<ScrollRotate3D
  initialRotateY={8}
  maxRotateY={-8}
  initialRotateX={-3}
  maxRotateX={3}
  minScale={0.97}
  maxScale={1.01}
  smoothing={0.1}
>
```

### Subtle Professional Look

For minimal, professional animation:

```tsx
<ScrollRotate3D
  initialRotateY={6}
  maxRotateY={-6}
  initialRotateX={-2}
  maxRotateX={2}
  minScale={0.98}
  maxScale={1}
>
```

## Complete Example Component

Here's a complete example you can copy-paste:

```tsx
'use client'

import React from 'react'
import { FadeIn, ScrollRotate3D } from './animations'

function EnhancedFeatureSection() {
  return (
    <div className="py-16 md:py-24 lg:py-32">
      <div className="max-w-7xl mx-auto px-6 md:px-8 lg:px-12">
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-12 md:gap-16 lg:gap-24 items-center">
          {/* Content */}
          <div className="flex flex-col">
            <FadeIn direction="left">
              <h3 className="text-2xl sm:text-3xl md:text-4xl lg:text-5xl font-serif font-semibold">
                Your Feature Title
              </h3>
            </FadeIn>
            <FadeIn delay={0.1} direction="left">
              <p className="mt-6 text-lg md:text-xl">
                Feature description goes here
              </p>
            </FadeIn>
          </div>

          {/* Image with 3D scroll rotation */}
          <FadeIn direction="right" delay={0.2}>
            <ScrollRotate3D
              initialRotateY={12}
              maxRotateY={-12}
              initialRotateX={-4}
              maxRotateX={4}
            >
              <div
                className="rounded-2xl overflow-hidden"
                style={{
                  backgroundColor: 'hsl(var(--card))',
                  border: '1px solid hsl(var(--border))',
                }}
              >
                <img
                  src="/your-image.png"
                  alt="Feature screenshot"
                  className="w-full h-auto"
                />
              </div>
            </ScrollRotate3D>
          </FadeIn>
        </div>
      </div>
    </div>
  )
}
```

## Testing

1. Add the component to any section
2. Scroll through the page slowly
3. Observe the smooth 3D rotation effect
4. Adjust values if needed

### Visual Testing Checklist

- [ ] Rotation direction feels natural
- [ ] Animation is smooth (no jitter)
- [ ] Scale effect is subtle but noticeable
- [ ] Works well with existing FadeIn animations
- [ ] Performs well on slower devices
- [ ] No layout shifts or jumps

## Performance Considerations

The component is already optimized, but keep in mind:

1. **Limit instances per page**: 5-10 simultaneous instances is fine
2. **Simple children**: Complex DOM trees will still perform well, but simpler is better
3. **Mobile**: Consider reducing rotation angles on mobile for subtlety

### Mobile Optimization

For mobile-specific values, use Tailwind breakpoints or conditional rendering:

```tsx
const isMobile = typeof window !== 'undefined' && window.innerWidth < 768

<ScrollRotate3D
  initialRotateY={isMobile ? 8 : 12}
  maxRotateY={isMobile ? -8 : -12}
  minScale={isMobile ? 0.98 : 0.95}
>
```

Or use CSS media queries for conditional display:

```tsx
<ScrollRotate3D className="hidden md:block">
  {/* Only on desktop */}
</ScrollRotate3D>

<div className="md:hidden">
  {/* Static version on mobile */}
</div>
```

## Combining with Existing Patterns

### With Mouse Tracking (like SupportSection)

You can combine scroll rotation with mouse tracking:

```tsx
<ScrollRotate3D>
  <div
    onMouseMove={handleMouseMove}
    onMouseLeave={handleMouseLeave}
    style={{
      transformStyle: 'preserve-3d',
    }}
  >
    {/* Content gets both scroll AND mouse effects */}
  </div>
</ScrollRotate3D>
```

The scroll rotation will set the base transform, and mouse tracking can add additional rotation on top.

### With FadeIn

Already demonstrated above - FadeIn should wrap ScrollRotate3D:

```tsx
<FadeIn>
  <ScrollRotate3D>
    <Content />
  </ScrollRotate3D>
</FadeIn>
```

## Troubleshooting

**Q: Animation feels choppy**
- Adjust `smoothing` prop (lower = smoother, try 0.05-0.06)
- Check if other animations are running simultaneously

**Q: Rotation is too subtle/dramatic**
- Adjust `initialRotateY` and `maxRotateY` values
- Default is ±15deg, try ±20-25deg for more drama or ±5-8deg for subtlety

**Q: Scale effect not visible**
- Increase difference between `minScale` and `maxScale`
- Default range is 0.95-1.0, try 0.92-1.05 for more noticeable effect

**Q: Works in dev but not in production build**
- Component is client-side only ('use client' directive)
- Ensure Next.js static export supports client components

## Next Steps

1. Try the example file: Run the marketing dev server and visit the examples
2. Integrate into one section of WhySoulPlayer.tsx
3. Test and adjust values to your liking
4. Apply to other sections as desired
5. Consider adding to other pages (about, features, etc.)

For detailed API documentation, see `applications/marketing/src/components/animations/README.md`
