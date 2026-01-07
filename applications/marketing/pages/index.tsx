import { PremiumHero } from '@/components/PremiumHero'
import { ComparisonSection } from '@/components/ComparisonSection'
import { WhySoulPlayer } from '@/components/WhySoulPlayer'
import { SupportSection } from '@/components/SupportSection'
import { Footer } from '@/components/Footer'

export default function Home() {
  return (
    <main className="min-h-screen bg-background">
      <PremiumHero />
      <WhySoulPlayer />
      <ComparisonSection />
      <SupportSection />
      <Footer />
    </main>
  )
}
