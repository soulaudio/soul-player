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

        <div className="mt-16 text-center">
          <div className="inline-block bg-violet-950/30 border border-violet-500/30 rounded-2xl p-8 max-w-2xl">
            <h3 className="text-2xl font-bold mb-3">
              Own Your Music Library
            </h3>
            <p className="text-zinc-400">
              Soul Player combines the convenience of streaming services with the privacy
              of self-hosted solutions. No subscriptions, no tracking, no compromises.
            </p>
          </div>
        </div>
      </div>
    </section>
  )
}
