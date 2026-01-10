import type { Metadata } from 'next'
import '@/styles/globals.css'
import { GITHUB_REPO, DISCORD_INVITE } from '@/constants/links'

const SITE_URL = 'https://player.soulaudio.co'
const SITE_NAME = 'Soul Player'
const SITE_TITLE = 'Soul Player - Own Your Music. Free & Open Source Music Player'
const SITE_DESCRIPTION = 'Free, open-source music player that puts you in control. Local-first playback, multi-user streaming, bit-perfect audio, zero tracking. Own your music library forever.'

export const metadata: Metadata = {
  metadataBase: new URL(SITE_URL),
  title: {
    default: SITE_TITLE,
    template: '%s | Soul Player',
  },
  description: SITE_DESCRIPTION,
  keywords: [
    'music player',
    'open source',
    'local music',
    'FLAC player',
    'audiophile',
    'self-hosted',
    'music streaming',
    'bit-perfect',
    'lossless audio',
    'privacy',
    'no tracking',
    'free music player',
    'desktop music player',
    'cross-platform',
  ],
  authors: [{ name: 'Soul Audio' }],
  creator: 'Soul Audio',
  publisher: 'Soul Audio',
  robots: {
    index: true,
    follow: true,
    googleBot: {
      index: true,
      follow: true,
    },
  },
  openGraph: {
    type: 'website',
    locale: 'en_US',
    url: SITE_URL,
    siteName: SITE_NAME,
    title: SITE_TITLE,
    description: SITE_DESCRIPTION,
    images: [
      {
        url: '/og-image.png',
        width: 1200,
        height: 630,
        alt: 'Soul Player - Own Your Music',
      },
    ],
  },
  twitter: {
    card: 'summary_large_image',
    site: '@soulaudio',
    creator: '@soulaudio',
    title: SITE_TITLE,
    description: SITE_DESCRIPTION,
    images: ['/og-image.png'],
  },
  icons: {
    icon: [
      { url: '/favicon.ico', sizes: 'any' },
      { url: '/favicon.svg', type: 'image/svg+xml' },
    ],
    apple: '/apple-touch-icon.png',
  },
  manifest: '/site.webmanifest',
  applicationName: SITE_NAME,
  appleWebApp: {
    capable: true,
    statusBarStyle: 'black-translucent',
    title: SITE_NAME,
  },
  formatDetection: {
    telephone: false,
  },
  other: {
    'theme-color': '#7c3aed',
    'color-scheme': 'dark',
  },
}

export default function RootLayout({
  children,
}: {
  children: React.ReactNode
}) {
  return (
    <html lang="en" className="dark">
      <head>
        <link rel="preconnect" href="https://fonts.googleapis.com" />
        <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="anonymous" />
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
                GITHUB_REPO,
                DISCORD_INVITE,
              ],
            }),
          }}
        />
      </head>
      <body className="antialiased text-white" style={{ backgroundColor: 'hsl(250, 15%, 4%)' }}>
        {children}
      </body>
    </html>
  )
}
