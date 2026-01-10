'use client'

import React, { useState } from 'react'
import { FadeIn } from './animations/FadeIn'

interface SourceLink {
  name: string
  url: string
}

interface Problem {
  title: string
  description: string
  sources: SourceLink[]
}

function CritiqueItem({ title, description, sources }: Problem) {
  const [showAllSources, setShowAllSources] = useState(false)
  const visibleSources = showAllSources ? sources : sources.slice(0, 3)

  return (
    <div className="flex-shrink-0 w-full sm:w-[calc(33.333%-1rem)]">
      <h4
        className="text-base font-semibold mb-1"
        style={{ color: 'hsl(var(--foreground))' }}
      >
        {title}
      </h4>
      <p
        className="text-sm leading-relaxed mb-2"
        style={{ color: 'hsl(var(--muted-foreground))' }}
      >
        {description}
      </p>
      <div className="flex flex-wrap items-center gap-x-2 gap-y-1">
        {visibleSources.map((source, i) => (
          <a
            key={i}
            href={source.url}
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center gap-1 text-xs transition-opacity hover:opacity-70"
            style={{ color: 'hsl(var(--muted-foreground) / 0.5)' }}
          >
            <svg className="w-2.5 h-2.5" fill="none" stroke="currentColor" strokeWidth={2} viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" d="M13.5 6H5.25A2.25 2.25 0 003 8.25v10.5A2.25 2.25 0 005.25 21h10.5A2.25 2.25 0 0018 18.75V10.5m-10.5 6L21 3m0 0h-5.25M21 3v5.25" />
            </svg>
            {source.name}
          </a>
        ))}
        {sources.length > 3 && !showAllSources && (
          <button
            onClick={() => setShowAllSources(true)}
            className="text-xs transition-opacity hover:opacity-70"
            style={{ color: 'hsl(var(--muted-foreground) / 0.5)' }}
          >
            +{sources.length - 3}
          </button>
        )}
      </div>
    </div>
  )
}

export function StreamingCritique() {
  const [currentPage, setCurrentPage] = useState(0)

  const problems: Problem[] = [
    {
      title: 'Artists earn poverty wages',
      description: 'Streaming pays $3-6 per 1,000 plays. Most artists make under $5,000/year. $47M withheld from small artists in 2024...',
      sources: [
        { name: 'TechCrunch', url: 'https://techcrunch.com/2025/03/11/spotify-says-its-payouts-are-getting-better-but-artists-still-disagree/' },
        { name: 'Disc Makers', url: 'https://blog.discmakers.com/2025/04/how-small-artists-got-robbed/' },
        { name: 'Variety', url: 'https://variety.com/2025/digital/news/spotify-paid-4-billion-music-songwriters-struggling-1236334752/' },
        { name: 'Music Ally', url: 'https://musically.com/2025/12/16/spotify-defends-1000-stream-royalties-threshold-after-critical-report/' },
        { name: 'Regulatory Review', url: 'https://www.theregreview.org/2024/05/30/stern-the-inequalities-of-digital-music-streaming/' },
      ],
    },
    {
      title: 'Your library vanishes overnight',
      description: 'Licensing disputes mean songs disappear. BTS, BLACKPINK lost hundreds of millions of streams. Playlists break...',
      sources: [
        { name: 'Irish Times', url: 'https://www.irishtimes.com/culture/music/2025/05/22/the-mystery-of-spotifys-disappearing-songs-what-we-lose-when-artists-erase-their-music/' },
        { name: 'Kluwer Copyright', url: 'https://legalblogs.wolterskluwer.com/copyright-blog/music-streaming-debates-2025-roundup-wrap-up-for-the-streaming-services-as-we-know-them-part-1/' },
        { name: 'Xposure Music', url: 'https://info.xposuremusic.com/article/why-some-songs-disappear-from-streaming-platforms-and-how-to-avoid-it' },
        { name: 'NexaTunes', url: 'https://blog.nexatunes.com/when-streams-disappear-what-artists-should-know-about-spotifys-artificial-stream-removals/' },
        { name: 'Alan Cross', url: 'https://www.ajournalofmusicalthings.com/streaming-music-services-have-access-to-176-million-songs-yet-some-tracks-are-still-missing-and-lost-why/' },
      ],
    },
    {
      title: 'Artists fleeing in 2025',
      description: 'Massive Attack, King Gizzard, Godspeed, Sylvan Esso left. Largest artist boycott in streaming history...',
      sources: [
        { name: 'Stereogum', url: 'https://stereogum.com/2482225/19-acts-who-pulled-their-music-off-spotify-this-year/lists/year-in-review/2025-in-review' },
        { name: 'CBC News', url: 'https://www.cbc.ca/news/entertainment/spotify-streaming-platform-controversy-2025-9.6996115' },
        { name: 'Far Out', url: 'https://faroutmagazine.co.uk/new-years-resolutions-why-leaving-spotify/' },
        { name: 'Wikipedia', url: 'https://en.wikipedia.org/wiki/Criticism_of_Spotify' },
        { name: 'Epigram', url: 'https://epigram.org.uk/spotify-unwrapped-the-truth-behind-the-popular-streaming-platform/' },
      ],
    },
    {
      title: 'Pay-to-play "Discovery Mode"',
      description: 'Artists pay 30% royalty cut to appear in recommendations. Class action lawsuit calls it "modern payola"...',
      sources: [
        { name: 'Rolling Stone', url: 'https://www.rollingstone.com/music/music-news/class-action-lawsuit-spotify-payola-discovery-mode-1235460448/' },
        { name: 'Billboard', url: 'https://www.billboard.com/pro/spotify-lawsuit-discovery-mode-modern-payola/' },
        { name: 'Digital Music News', url: 'https://www.digitalmusicnews.com/2025/11/05/spotify-accused-of-payola-in-class-action-lawsuit/' },
        { name: 'Recording Academy', url: 'https://www.recordingacademy.com/advocacy/news/does-spotifys-new-discovery-mode-resemble-anti-creator-payola' },
        { name: 'Music Tech', url: 'https://musictech.solutions/2025/11/10/the-digital-end-cap-how-spotifys-discovery-mode-turned-payola-into-personalization/' },
        { name: 'Making A Scene', url: 'https://www.makingascene.org/spotifys-discovery-mode-the-new-payola-hurting-indie-artists/' },
      ],
    },
    {
      title: 'Algorithms push paid content',
      description: 'Your "personalized" playlists filled with major label artists who paid for placement. Drake, Bieber appear regardless of taste...',
      sources: [
        { name: 'Noise11', url: 'https://www.noise11.com/news/spotify-discovery-mode-payola-fake-streams-transparency-20251106' },
        { name: 'Law Commentary', url: 'https://www.lawcommentary.com/articles/spotify-faces-new-class-action-lawsuit-alleging-payola-style-practices-in-discovery-mode' },
        { name: 'Hit Channel', url: 'https://hit-channel.com/spotify-discovery-mode-lawsuit-2025/' },
        { name: 'Archyde', url: 'https://www.archyde.com/spotify-responds-to-payola-lawsuit-regarding-discovery-mode/' },
        { name: 'Music Forem', url: 'https://music.forem.com/ronnie_pye_/how-streaming-platforms-engineered-their-own-piracy-problem-a-data-story-4mkl' },
      ],
    },
    {
      title: 'Indies invisible without paying',
      description: 'Without Discovery Mode, indie artists disappear from recommendations. Pay 30% or become algorithmically invisible...',
      sources: [
        { name: 'Making A Scene', url: 'https://www.makingascene.org/spotifys-discovery-mode-the-new-payola-hurting-indie-artists/' },
        { name: 'Music Tech', url: 'https://musictech.solutions/2025/11/10/the-digital-end-cap-how-spotifys-discovery-mode-turned-payola-into-personalization/' },
        { name: 'Regulatory Review', url: 'https://www.theregreview.org/2024/05/30/stern-the-inequalities-of-digital-music-streaming/' },
        { name: 'New School Free Press', url: 'https://www.newschoolfreepress.com/2025/12/08/spotify-isnt-your-friend-how-the-platform-takes-your-money-while-artists-pay-the-price/' },
      ],
    },
    {
      title: 'AI stealing from musicians',
      description: 'AI "artists" hit millions of streams. Every AI stream takes money from real musicians\' royalty pool...',
      sources: [
        { name: 'New School Free Press', url: 'https://www.newschoolfreepress.com/2025/12/08/spotify-isnt-your-friend-how-the-platform-takes-your-money-while-artists-pay-the-price/' },
        { name: 'Rolling Stone', url: 'https://council.rollingstone.com/blog/spotify-under-fire-over-investments-and-artist-pay/' },
        { name: 'Music Ally', url: 'https://musically.com/2025/12/16/spotify-defends-1000-stream-royalties-threshold-after-critical-report/' },
        { name: 'Rebel Music', url: 'https://rebelmusicdistribution.com/2025/12/25/how-much-spotify-will-pay-in-2026/' },
      ],
    },
    {
      title: 'Billions in fake streams',
      description: 'Lawsuits allege billions of bot streams benefit certain artists. Charts manipulated, royalties diverted...',
      sources: [
        { name: 'Noise11', url: 'https://www.noise11.com/news/spotify-discovery-mode-payola-fake-streams-transparency-20251106' },
        { name: 'CBC News', url: 'https://www.cbc.ca/news/entertainment/spotify-streaming-platform-controversy-2025-9.6996115' },
        { name: 'NexaTunes', url: 'https://blog.nexatunes.com/when-streams-disappear-what-artists-should-know-about-spotifys-artificial-stream-removals/' },
        { name: 'Digital Music News', url: 'https://www.digitalmusicnews.com/2025/11/05/spotify-accused-of-payola-in-class-action-lawsuit/' },
      ],
    },
    {
      title: 'You own nothing',
      description: 'Stop paying and everything disappears. Travel abroad, songs become "unavailable in your region"...',
      sources: [
        { name: 'DEV Community', url: 'https://dev.to/david_whitney/music-streaming-and-the-disappearing-records-513b' },
        { name: 'Windows Central', url: 'https://www.windowscentral.com/buying-music-vs-streaming-which-better-you' },
        { name: 'MakeUseOf', url: 'https://www.makeuseof.com/prefer-owning-music-over-streaming/' },
        { name: 'Mid Theory', url: 'https://mid-theory.com/2025/05/27/how-to-disappear-in-the-age-of-streaming/' },
      ],
    },
    {
      title: 'Profits fund weapons',
      description: 'CEO invested $702M in AI combat drones. Union: "a warmonger who pays artists poverty wages"...',
      sources: [
        { name: 'Rolling Stone', url: 'https://council.rollingstone.com/blog/spotify-under-fire-over-investments-and-artist-pay/' },
        { name: 'CBC News', url: 'https://www.cbc.ca/news/entertainment/spotify-streaming-platform-controversy-2025-9.6996115' },
        { name: 'New School Free Press', url: 'https://www.newschoolfreepress.com/2025/12/08/spotify-isnt-your-friend-how-the-platform-takes-your-money-while-artists-pay-the-price/' },
        { name: 'Wikipedia', url: 'https://en.wikipedia.org/wiki/Criticism_of_Spotify' },
      ],
    },
    {
      title: 'Services die, libraries gone',
      description: 'Rdio, Google Play Music—gone forever. Years of playlists and saved albums disappeared...',
      sources: [
        { name: 'DEV Community', url: 'https://dev.to/david_whitney/music-streaming-and-the-disappearing-records-513b' },
        { name: 'Teufel Blog', url: 'https://blog.teufelaudio.com/buying-music-online/' },
        { name: 'Medium', url: 'https://medium.com/@HHintze/cds-vs-streaming-why-physical-media-is-still-a-smart-investment-61e9c3ca1c54' },
      ],
    },
    {
      title: 'Every listen tracked',
      description: 'Every play, skip, pause logged. Data sold to advertisers. No opt-out for core tracking...',
      sources: [
        { name: 'Wikipedia', url: 'https://en.wikipedia.org/wiki/Criticism_of_Spotify' },
        { name: 'Epigram', url: 'https://epigram.org.uk/spotify-unwrapped-the-truth-behind-the-popular-streaming-platform/' },
        { name: 'New School Free Press', url: 'https://www.newschoolfreepress.com/2025/12/08/spotify-isnt-your-friend-how-the-platform-takes-your-money-while-artists-pay-the-price/' },
      ],
    },
  ]

  const itemsPerPage = 3
  const totalPages = Math.ceil(problems.length / itemsPerPage)
  const startIndex = currentPage * itemsPerPage
  const visibleProblems = problems.slice(startIndex, startIndex + itemsPerPage)

  const goToPrev = () => setCurrentPage((p) => Math.max(0, p - 1))
  const goToNext = () => setCurrentPage((p) => Math.min(totalPages - 1, p + 1))

  const comparisonExamples = [
    {
      scenario: 'Listen to album 100 times',
      streaming: '$12/month. Artist gets $0.30. 10 years = $1,440 paid, own nothing.',
      owning: '$10 once. Artist gets $8-9. Own forever.',
    },
    {
      scenario: 'Artist removes music',
      streaming: 'Gone. Playlists break. Never hear it again.',
      owning: 'Nothing changes. Files are yours.',
    },
    {
      scenario: 'Cancel subscription',
      streaming: 'Lose everything. Years of playlists—gone.',
      owning: 'Music still there. No subscription.',
    },
  ]

  const comparisonSources = [
    { name: 'WXPN', url: 'https://xpn.org/2024/01/25/streaming-versus-owning-music-consumer-trends/' },
    { name: 'Teufel', url: 'https://blog.teufelaudio.com/buying-music-online/' },
    { name: 'Medium', url: 'https://medium.com/@HHintze/cds-vs-streaming-why-physical-media-is-still-a-smart-investment-61e9c3ca1c54' },
    { name: 'Royalty Exchange', url: 'https://royaltyexchange.com/blog/the-impact-of-streaming-services-on-music-royalties' },
  ]

  return (
    <div className="py-8 md:py-12">
      <div className="max-w-7xl mx-auto px-6 md:px-8 lg:px-12">
        {/* Header */}
        <FadeIn>
          <div className="mb-6">
            <h3
              className="text-2xl sm:text-3xl md:text-4xl font-serif font-bold tracking-tight mb-2"
              style={{ color: 'hsl(var(--foreground))' }}
            >
              Streaming is Broken
            </h3>
            <p
              className="text-sm leading-relaxed max-w-2xl"
              style={{ color: 'hsl(var(--muted-foreground))' }}
            >
              In 2025, artists fled platforms, royalties were stolen, and libraries vanished.
            </p>
          </div>
        </FadeIn>

        {/* Carousel */}
        <FadeIn delay={0.1}>
          <div className="relative">
            {/* Items */}
            <div className="flex gap-6 mb-4">
              {visibleProblems.map((problem, i) => (
                <CritiqueItem key={startIndex + i} {...problem} />
              ))}
            </div>

            {/* Navigation */}
            <div className="flex items-center gap-3">
              <button
                onClick={goToPrev}
                disabled={currentPage === 0}
                className="p-1.5 rounded-md transition-all disabled:opacity-20"
                style={{
                  color: 'hsl(var(--muted-foreground))',
                  backgroundColor: currentPage === 0 ? 'transparent' : 'hsl(var(--muted) / 0.5)',
                }}
              >
                <svg className="w-4 h-4" fill="none" stroke="currentColor" strokeWidth={2} viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" d="M15.75 19.5L8.25 12l7.5-7.5" />
                </svg>
              </button>

              <div className="flex gap-1.5">
                {Array.from({ length: totalPages }).map((_, i) => (
                  <button
                    key={i}
                    onClick={() => setCurrentPage(i)}
                    className="w-1.5 h-1.5 rounded-full transition-all"
                    style={{
                      backgroundColor: i === currentPage
                        ? 'hsl(var(--foreground))'
                        : 'hsl(var(--muted-foreground) / 0.3)',
                    }}
                  />
                ))}
              </div>

              <button
                onClick={goToNext}
                disabled={currentPage === totalPages - 1}
                className="p-1.5 rounded-md transition-all disabled:opacity-20"
                style={{
                  color: 'hsl(var(--muted-foreground))',
                  backgroundColor: currentPage === totalPages - 1 ? 'transparent' : 'hsl(var(--muted) / 0.5)',
                }}
              >
                <svg className="w-4 h-4" fill="none" stroke="currentColor" strokeWidth={2} viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" d="M8.25 4.5l7.5 7.5-7.5 7.5" />
                </svg>
              </button>

              <span
                className="text-xs ml-2"
                style={{ color: 'hsl(var(--muted-foreground) / 0.5)' }}
              >
                {currentPage + 1}/{totalPages}
              </span>
            </div>
          </div>
        </FadeIn>

        {/* Comparison */}
        <FadeIn delay={0.2}>
          <div className="mt-12 mb-4">
            <h3
              className="text-xl sm:text-2xl font-serif font-bold tracking-tight mb-4"
              style={{ color: 'hsl(var(--foreground))' }}
            >
              What This Means
            </h3>
            {comparisonExamples.map((example, i) => (
              <div
                key={i}
                className="mb-4 pb-4"
                style={{ borderBottom: '1px solid hsl(var(--border) / 0.3)' }}
              >
                <p
                  className="text-sm font-medium mb-2"
                  style={{ color: 'hsl(var(--foreground))' }}
                >
                  {example.scenario}
                </p>
                <div className="grid grid-cols-2 gap-4">
                  <div>
                    <span
                      className="text-xs uppercase tracking-wider"
                      style={{ color: 'hsl(var(--muted-foreground) / 0.5)' }}
                    >
                      Streaming
                    </span>
                    <p
                      className="text-sm mt-0.5"
                      style={{ color: 'hsl(var(--muted-foreground))' }}
                    >
                      {example.streaming}
                    </p>
                  </div>
                  <div>
                    <span
                      className="text-xs uppercase tracking-wider"
                      style={{ color: 'hsl(var(--muted-foreground) / 0.5)' }}
                    >
                      Owning
                    </span>
                    <p
                      className="text-sm mt-0.5"
                      style={{ color: 'hsl(var(--muted-foreground))' }}
                    >
                      {example.owning}
                    </p>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </FadeIn>

        {/* Sources */}
        <FadeIn delay={0.3}>
          <div className="flex flex-wrap gap-x-3 gap-y-1">
            {comparisonSources.map((source, i) => (
              <a
                key={i}
                href={source.url}
                target="_blank"
                rel="noopener noreferrer"
                className="inline-flex items-center gap-1 text-xs transition-opacity hover:opacity-70"
                style={{ color: 'hsl(var(--muted-foreground) / 0.5)' }}
              >
                <svg className="w-2.5 h-2.5" fill="none" stroke="currentColor" strokeWidth={2} viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" d="M13.5 6H5.25A2.25 2.25 0 003 8.25v10.5A2.25 2.25 0 005.25 21h10.5A2.25 2.25 0 0018 18.75V10.5m-10.5 6L21 3m0 0h-5.25M21 3v5.25" />
                </svg>
                {source.name}
              </a>
            ))}
          </div>
        </FadeIn>
      </div>
    </div>
  )
}
