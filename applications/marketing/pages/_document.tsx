import { Html, Head, Main, NextScript } from 'next/document'

export default function Document() {
  return (
    <Html lang="en" className="dark">
      <Head>
        <meta charSet="utf-8" />
        <meta name="description" content="Local-first, cross-platform music player with multi-user server streaming and embedded hardware support" />
        <meta property="og:title" content="Soul Player - Your Music, Your Way" />
        <meta property="og:description" content="Self-hosted music player with multi-user streaming. No subscriptions, no tracking." />
        <meta property="og:type" content="website" />
        <meta property="og:url" content="https://player.soulaudio.co" />
        <meta name="twitter:card" content="summary_large_image" />
        <meta name="twitter:title" content="Soul Player" />
        <meta name="twitter:description" content="Local-first, cross-platform music player" />
        <link rel="icon" href="/favicon.ico" />
      </Head>
      <body className="antialiased text-white" style={{ backgroundColor: 'hsl(250, 15%, 4%)' }}>
        <Main />
        <NextScript />
      </body>
    </Html>
  )
}
