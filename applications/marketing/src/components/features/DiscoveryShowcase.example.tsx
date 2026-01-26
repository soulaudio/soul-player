/**
 * Example usage of DiscoveryShowcase component
 *
 * This component showcases metadata integration from external music services
 * (Discogs, Bandcamp, MusicBrainz, AcoustID) with animated data flow visualization
 * and before/after metadata enhancement demonstration.
 *
 * Usage:
 * ```tsx
 * import { DiscoveryShowcase } from '@soul-player/marketing/components'
 *
 * export default function MarketingPage() {
 *   return (
 *     <main>
 *       <Hero />
 *       <DiscoveryShowcase />
 *       <Footer />
 *     </main>
 *   )
 * }
 * ```
 *
 * Features:
 * - 3D rotation effect based on scroll position
 * - Automatic cycling through service badges (3s intervals)
 * - Before/after metadata comparison (toggles every 4s)
 * - Animated data flow lines from services to metadata
 * - Floating particles during enhanced mode
 * - Responsive grid layout (stacks on mobile)
 *
 * Visual Structure:
 * - Left side: Service badges with data flow animation
 * - Center: Rotating hub icon
 * - Right side: Album card with before/after metadata
 * - Bottom: Feature highlights (3 cards)
 *
 * Dependencies:
 * - framer-motion (animations)
 * - lucide-react (icons)
 * - react-awesome-reveal (FadeIn animation)
 */

import { DiscoveryShowcase } from './DiscoveryShowcase'

export default function Example() {
  return (
    <div className="min-h-screen bg-zinc-950">
      <DiscoveryShowcase />
    </div>
  )
}
