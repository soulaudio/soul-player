'use client'

import { ComparisonTable } from './ComparisonTable'

export function ComparisonSection() {
  return (
    <section className="py-24 bg-zinc-950">
      <div className="container mx-auto px-6">
        <div className="text-center mb-16">
          <h2 className="text-5xl font-serif font-bold mb-4">
            Not Just Another Music Player
          </h2>
          <p className="text-xl text-zinc-400 max-w-3xl mx-auto">
            Compare Soul Player to streaming services and self-hosted alternatives
          </p>
        </div>

        <div className="max-w-5xl mx-auto">
          <ComparisonTable />
        </div>
      </div>
    </section>
  )
}
