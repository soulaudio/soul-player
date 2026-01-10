import { Html, Head, Main, NextScript } from 'next/document'

const SITE_URL = 'https://player.soulaudio.co'
const SITE_NAME = 'Soul Player'
const SITE_TITLE = 'Soul Player - Own Your Music. Free & Open Source Music Player'
const SITE_DESCRIPTION = 'Free, open-source music player that puts you in control. Local-first playback, multi-user streaming, bit-perfect audio, zero tracking. Own your music library forever.'

export default function Document() {
  return (
    <Html lang="en" className="dark">
      <Head>
        <meta charSet="utf-8" />

        {/* Primary Meta Tags */}
        <meta name="description" content={SITE_DESCRIPTION} />
        <meta name="keywords" content="music player, open source, local music, FLAC player, audiophile, self-hosted, music streaming, bit-perfect, lossless audio, privacy, no tracking, free music player, desktop music player, cross-platform" />
        <meta name="author" content="Soul Audio" />
        <meta name="robots" content="index, follow" />
        <meta name="googlebot" content="index, follow" />

        {/* Canonical */}
        <link rel="canonical" href={SITE_URL} />

        {/* Open Graph / Facebook */}
        <meta property="og:type" content="website" />
        <meta property="og:url" content={SITE_URL} />
        <meta property="og:site_name" content={SITE_NAME} />
        <meta property="og:title" content={SITE_TITLE} />
        <meta property="og:description" content={SITE_DESCRIPTION} />
        <meta property="og:image" content={`${SITE_URL}/og-image.png`} />
        <meta property="og:image:width" content="1200" />
        <meta property="og:image:height" content="630" />
        <meta property="og:image:alt" content="Soul Player - Own Your Music" />
        <meta property="og:locale" content="en_US" />

        {/* Twitter */}
        <meta name="twitter:card" content="summary_large_image" />
        <meta name="twitter:site" content="@soulaudio" />
        <meta name="twitter:creator" content="@soulaudio" />
        <meta name="twitter:title" content={SITE_TITLE} />
        <meta name="twitter:description" content={SITE_DESCRIPTION} />
        <meta name="twitter:image" content={`${SITE_URL}/og-image.png`} />
        <meta name="twitter:image:alt" content="Soul Player - Own Your Music" />

        {/* Theme & App */}
        <meta name="theme-color" content="#7c3aed" />
        <meta name="color-scheme" content="dark" />
        <meta name="application-name" content={SITE_NAME} />
        <meta name="apple-mobile-web-app-title" content={SITE_NAME} />
        <meta name="apple-mobile-web-app-capable" content="yes" />
        <meta name="apple-mobile-web-app-status-bar-style" content="black-translucent" />
        <meta name="mobile-web-app-capable" content="yes" />
        <meta name="format-detection" content="telephone=no" />

        {/* Favicon & Icons */}
        <link rel="icon" href="/favicon.ico" sizes="any" />
        <link rel="icon" href="/favicon.svg" type="image/svg+xml" />
        <link rel="apple-touch-icon" href="/apple-touch-icon.png" />
        <link rel="manifest" href="/site.webmanifest" />

        {/* Preconnect for performance */}
        <link rel="preconnect" href="https://fonts.googleapis.com" />
        <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="anonymous" />

        {/* Structured Data - JSON-LD */}
        <script
          type="application/ld+json"
          dangerouslySetInnerHTML={{
            __html: JSON.stringify({
              '@context': 'https://schema.org',
              '@type': 'SoftwareApplication',
              name: SITE_NAME,
              description: SITE_DESCRIPTION,
              url: SITE_URL,
              applicationCategory: 'MultimediaApplication',
              operatingSystem: 'Windows, macOS, Linux',
              offers: {
                '@type': 'Offer',
                price: '0',
                priceCurrency: 'USD',
              },
              aggregateRating: {
                '@type': 'AggregateRating',
                ratingValue: '5',
                ratingCount: '1',
              },
              author: {
                '@type': 'Organization',
                name: 'Soul Audio',
                url: SITE_URL,
              },
              featureList: [
                'Local-first music playback',
                'Multi-user streaming server',
                'Bit-perfect audio output',
                'FLAC, MP3, AAC, OGG support',
                'Zero tracking and telemetry',
                'Open source and free forever',
                'Cross-platform (Windows, macOS, Linux)',
                'Self-hosted option',
              ],
            }),
          }}
        />

        {/* Organization Schema */}
        <script
          type="application/ld+json"
          dangerouslySetInnerHTML={{
            __html: JSON.stringify({
              '@context': 'https://schema.org',
              '@type': 'Organization',
              name: 'Soul Audio',
              url: SITE_URL,
              logo: `${SITE_URL}/soul-audio-logo.svg`,
              sameAs: [
                'https://github.com/soulaudio/soul-player',
                'https://discord.gg/soulplayer',
              ],
            }),
          }}
        />
      </Head>
      <body className="antialiased text-white" style={{ backgroundColor: 'hsl(250, 15%, 4%)' }}>
        <Main />
        <NextScript />
      </body>
    </Html>
  )
}
