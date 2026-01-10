import Head from 'next/head'
import { PremiumHero } from '@/components/PremiumHero'
import { ComparisonSection } from '@/components/ComparisonSection'
import { WhySoulPlayer } from '@/components/WhySoulPlayer'
import { Footer } from '@/components/Footer'

export default function Home() {
  return (
    <>
      <Head>
        <title>Soul Player - Own Your Music. Free & Open Source Music Player</title>
      </Head>
      <main className="min-h-screen bg-background">
        <PremiumHero />
        <WhySoulPlayer />
        <ComparisonSection />
        <Footer />
      </main>
    </>
  )
}
