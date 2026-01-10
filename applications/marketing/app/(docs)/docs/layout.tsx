import { ReactNode } from 'react'
import Link from 'next/link'
import { DocsSidebar } from './components/DocsSidebar'
import { DocsSearch } from './components/DocsSearch'
import {
  GITHUB_REPO,
  GITHUB_EDIT_DOCS,
  GITHUB_SPONSORS,
  DISCORD_INVITE,
} from '@/constants/links'

export const metadata = {
  title: {
    template: '%s | Soul Player Docs',
    default: 'Documentation',
  },
}

export const dynamic = 'force-static'

export default function DocsLayout({ children }: { children: ReactNode }) {
  return (
    <div className="min-h-screen bg-background">
      {/* Header - matches landing page style */}
      <header className="fixed top-0 left-0 right-0 z-50">
        <nav className="container mx-auto px-6 py-4 flex items-center justify-between">
          <div className="flex items-center gap-4">
            <Link
              href="/"
              className="font-serif text-lg text-foreground transition-colors duration-300"
              style={{ textShadow: '0 1px 2px hsl(var(--background) / 0.8)' }}
            >
              Soul Player
            </Link>
            <span className="text-xs px-2 py-1 rounded text-muted-foreground bg-muted">
              Docs
            </span>
          </div>
          <div className="flex items-center gap-3">
            <DocsSearch />
            <a
              href={DISCORD_INVITE}
              target="_blank"
              rel="noopener noreferrer"
              className="text-foreground transition-colors hover:opacity-80"
              aria-label="Discord"
            >
              <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 24 24">
                <path d="M20.317 4.37a19.791 19.791 0 0 0-4.885-1.515.074.074 0 0 0-.079.037c-.21.375-.444.864-.608 1.25a18.27 18.27 0 0 0-5.487 0 12.64 12.64 0 0 0-.617-1.25.077.077 0 0 0-.079-.037A19.736 19.736 0 0 0 3.677 4.37a.07.07 0 0 0-.032.027C.533 9.046-.32 13.58.099 18.057a.082.082 0 0 0 .031.057 19.9 19.9 0 0 0 5.993 3.03.078.078 0 0 0 .084-.028 14.09 14.09 0 0 0 1.226-1.994.076.076 0 0 0-.041-.106 13.107 13.107 0 0 1-1.872-.892.077.077 0 0 1-.008-.128 10.2 10.2 0 0 0 .372-.292.074.074 0 0 1 .077-.01c3.928 1.793 8.18 1.793 12.062 0a.074.074 0 0 1 .078.01c.12.098.246.198.373.292a.077.077 0 0 1-.006.127 12.299 12.299 0 0 1-1.873.892.077.077 0 0 0-.041.107c.36.698.772 1.362 1.225 1.993a.076.076 0 0 0 .084.028 19.839 19.839 0 0 0 6.002-3.03.077.077 0 0 0 .032-.054c.5-5.177-.838-9.674-3.549-13.66a.061.061 0 0 0-.031-.03zM8.02 15.33c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.956-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.956 2.418-2.157 2.418zm7.975 0c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.955-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.946 2.418-2.157 2.418z" />
              </svg>
            </a>
            <a
              href={GITHUB_REPO}
              target="_blank"
              rel="noopener noreferrer"
              className="text-foreground transition-colors hover:opacity-80"
              aria-label="GitHub"
            >
              <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 24 24">
                <path fillRule="evenodd" d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.531 1.032 1.531 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0112 6.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0022 12.017C22 6.484 17.522 2 12 2z" clipRule="evenodd" />
              </svg>
            </a>
          </div>
        </nav>
      </header>

      {/* Main layout container - flex with sidebars, padding for fixed header */}
      <div className="container mx-auto flex pt-16">
        {/* Left Sidebar - sticky */}
        <aside className="hidden md:block w-64 shrink-0 sticky top-16 h-[calc(100vh-4rem)] overflow-y-auto">
          <DocsSidebar />
        </aside>

        {/* Main content area */}
        <main className="flex-1 min-w-0">
          <div className="max-w-3xl mx-auto px-6 py-12">
            <article data-pagefind-body>{children}</article>
          </div>
        </main>

        {/* Right Sidebar - sticky */}
        <aside className="hidden lg:block w-64 shrink-0 sticky top-16 h-[calc(100vh-4rem)] overflow-y-auto">
          <div className="p-6 space-y-3">
            {/* Help us improve */}
            <a
              href={GITHUB_EDIT_DOCS}
              target="_blank"
              rel="noopener noreferrer"
              className="block rounded-lg p-4 bg-muted/20 transition-all duration-200 hover:bg-muted/40 hover:scale-[1.02]"
            >
              <div className="flex items-start justify-between mb-2">
                <h4 className="text-sm font-medium text-foreground">
                  Help us improve
                </h4>
                <svg className="w-4 h-4 text-muted-foreground" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
                </svg>
              </div>
              <p className="text-sm text-muted-foreground">
                Edit this page on GitHub
              </p>
            </a>

            {/* Join Discord */}
            <a
              href={DISCORD_INVITE}
              target="_blank"
              rel="noopener noreferrer"
              className="block rounded-lg p-4 bg-muted/20 transition-all duration-200 hover:bg-muted/40 hover:scale-[1.02]"
            >
              <div className="flex items-start justify-between mb-2">
                <h4 className="text-sm font-medium text-foreground">
                  Join the community
                </h4>
                <svg className="w-4 h-4 text-muted-foreground" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
                </svg>
              </div>
              <p className="text-sm text-muted-foreground">
                Chat with us on Discord
              </p>
            </a>

            {/* Star on GitHub */}
            <a
              href={GITHUB_REPO}
              target="_blank"
              rel="noopener noreferrer"
              className="block rounded-lg p-4 bg-muted/20 transition-all duration-200 hover:bg-muted/40 hover:scale-[1.02]"
            >
              <div className="flex items-start justify-between mb-2">
                <h4 className="text-sm font-medium text-foreground">
                  Support the project
                </h4>
                <svg className="w-4 h-4 text-muted-foreground" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
                </svg>
              </div>
              <p className="text-sm text-muted-foreground">
                Star us on GitHub
              </p>
            </a>

            {/* Community tier */}
            <a
              href={GITHUB_SPONSORS}
              target="_blank"
              rel="noopener noreferrer"
              className="block rounded-lg p-4 bg-gradient-to-br from-primary/20 to-primary/5 border border-primary/20 transition-all duration-200 hover:from-primary/30 hover:to-primary/10 hover:scale-[1.02]"
            >
              <div className="flex items-start justify-between mb-2">
                <h4 className="text-sm font-medium text-foreground">
                  Community Tier
                </h4>
                <svg className="w-4 h-4 text-primary" fill="currentColor" viewBox="0 0 24 24">
                  <path d="M12 21.35l-1.45-1.32C5.4 15.36 2 12.28 2 8.5 2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5c0 3.78-3.4 6.86-8.55 11.54L12 21.35z" />
                </svg>
              </div>
              <p className="text-sm text-muted-foreground">
                Become a sponsor
              </p>
            </a>
          </div>
        </aside>
      </div>
    </div>
  )
}
