import { PremiumHero } from '@/components/PremiumHero'
import { FeaturesSection } from '@/components/FeaturesSection'
import { ComparisonSection } from '@/components/ComparisonSection'
import { DiscoveryExplainer } from '@/components/DiscoveryExplainer'
import { ComingSoonSection } from '@/components/ComingSoonSection'
import { Footer } from '@/components/Footer'

export default function Home() {
  return (
    <main className="min-h-screen" style={{ backgroundColor: 'hsl(250, 15%, 4%)' }}>
      <PremiumHero />
      <FeaturesSection />
      <ComparisonSection />
      <DiscoveryExplainer />
      <ComingSoonSection />
      <Footer />
    </main>
  )
}
