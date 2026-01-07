import React from 'react'

const config = {
  logo: <span className="font-bold text-xl">Soul Player</span>,
  project: {
    link: 'https://github.com/soulaudio/soul-player',
  },
  docsRepositoryBase: 'https://github.com/soulaudio/soul-player/tree/main/applications/marketing',
  footer: {
    text: (
      <span>
        {new Date().getFullYear()} © Soul Player
      </span>
    ),
  },
  useNextSeoProps() {
    return {
      titleTemplate: '%s – Soul Player'
    }
  },
  head: (
    <>
      <meta name="viewport" content="width=device-width, initial-scale=1.0" />
      <meta property="og:title" content="Soul Player" />
      <meta property="og:description" content="Local-first, cross-platform music player" />
    </>
  ),
  primaryHue: 260,
  darkMode: true,
  nextThemes: {
    defaultTheme: 'dark',
  }
}

export default config
