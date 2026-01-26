/**
 * Example usage of MobileShowcase component
 *
 * This shows how to integrate the MobileShowcase into the WhySoulPlayer section
 * to replace the PlaceholderImage with an interactive 3D device showcase.
 */

import { MobileShowcase } from './MobileShowcase'

// Example: Replace the mobile section in WhySoulPlayer.tsx
export function MobileFeatureSection() {
  return (
    <div className="py-16 md:py-24 lg:py-32">
      <div className="max-w-7xl mx-auto px-6 md:px-8 lg:px-12">
        {/* Replace the entire FeatureSection for mobile with this: */}
        <MobileShowcase />
      </div>
    </div>
  )
}

// Or integrate into existing FeatureSection layout:
export function IntegratedMobileSection() {
  return (
    <div className="py-16 md:py-24 lg:py-32">
      <div className="max-w-7xl mx-auto px-6 md:px-8 lg:px-12">
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-12 md:gap-16 lg:gap-24 items-center">
          {/* Left side: Text content */}
          <div className="flex flex-col">
            <h3 className="text-2xl sm:text-3xl md:text-4xl lg:text-5xl font-serif font-semibold tracking-tight leading-tight">
              Listen on{' '}
              <span className="text-transparent bg-clip-text" style={{
                backgroundImage: 'linear-gradient(135deg, hsl(var(--primary)) 0%, hsl(var(--accent)) 100%)',
                WebkitBackgroundClip: 'text',
                WebkitTextFillColor: 'transparent',
              }}>
                the Go
              </span>
            </h3>
            <p className="mt-6 text-lg md:text-xl leading-relaxed" style={{ color: 'hsl(var(--muted-foreground))' }}>
              Your music, everywhere. Native mobile apps and dedicated hardware—all connected to your Soul Player ecosystem.
            </p>
            <ul className="mt-8 space-y-4">
              <li className="flex items-start gap-3">
                <div className="mt-2.5 w-1.5 h-1.5 rounded-full shrink-0" style={{ backgroundColor: 'hsl(var(--primary))' }} />
                <p className="text-base md:text-lg leading-relaxed" style={{ color: 'hsl(var(--muted-foreground))' }}>
                  <span style={{ color: 'hsl(var(--foreground))' }} className="font-medium">iOS & Android apps.</span>{' '}
                  Native mobile apps synced with your library and server.
                </p>
              </li>
              <li className="flex items-start gap-3">
                <div className="mt-2.5 w-1.5 h-1.5 rounded-full shrink-0" style={{ backgroundColor: 'hsl(var(--primary))' }} />
                <p className="text-base md:text-lg leading-relaxed" style={{ color: 'hsl(var(--muted-foreground))' }}>
                  <span style={{ color: 'hsl(var(--foreground))' }} className="font-medium">Offline sync.</span>{' '}
                  Download playlists for airplane mode and data-free listening.
                </p>
              </li>
              <li className="flex items-start gap-3">
                <div className="mt-2.5 w-1.5 h-1.5 rounded-full shrink-0" style={{ backgroundColor: 'hsl(var(--primary))' }} />
                <p className="text-base md:text-lg leading-relaxed" style={{ color: 'hsl(var(--muted-foreground))' }}>
                  <span style={{ color: 'hsl(var(--foreground))' }} className="font-medium">Physical DAP.</span>{' '}
                  E-Ink digital audio player—dedicated hardware for purists.
                </p>
              </li>
            </ul>
          </div>

          {/* Right side: 3D Device Showcase */}
          <div>
            <MobileShowcase />
          </div>
        </div>
      </div>
    </div>
  )
}
