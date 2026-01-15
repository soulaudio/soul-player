import { PremiumHero } from '@/components/PremiumHero'
import { ComparisonSection } from '@/components/ComparisonSection'
import { WhySoulPlayer } from '@/components/WhySoulPlayer'
import { Footer } from '@/components/Footer'
import { StickyHeader } from '@/components/StickyHeader'

export default function Home() {
  return (
    <>
      <StickyHeader />
      <main className="min-h-screen bg-background overflow-x-hidden">
        <PremiumHero />
        <WhySoulPlayer />
        <ComparisonSection />
        <Footer />
      </main>
    </>
  )
}
