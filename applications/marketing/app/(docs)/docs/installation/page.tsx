import { GITHUB_RELEASES } from '@/constants/links'

export const metadata = {
  title: 'Installation',
  description: 'Install Soul Player on Windows, macOS, or Linux. Build from source instructions.',
}

export const dynamic = 'force-static'

export default function InstallationPage() {
  return (
    <>
      <h1
        className="text-4xl font-serif mb-6"
        style={{ color: 'hsl(var(--foreground))' }}
      >
        Installation
      </h1>

      <p
        className="text-lg mb-8"
        style={{ color: 'hsl(var(--muted-foreground))' }}
      >
        Soul Player is available for Windows, macOS, and Linux.
      </p>

      <h2
        className="text-2xl font-serif mb-4 mt-10 pb-2 border-b"
        style={{ color: 'hsl(var(--foreground))', borderColor: 'hsl(var(--border))' }}
      >
        Downloads
      </h2>

      {/* Windows */}
      <h3
        className="text-xl font-medium mb-3 mt-8"
        style={{ color: 'hsl(var(--foreground))' }}
      >
        Windows
      </h3>
      <p className="mb-3" style={{ color: 'hsl(var(--muted-foreground))' }}>
        Download the latest{' '}
        <code
          className="px-1.5 py-0.5 rounded text-sm"
          style={{ backgroundColor: 'hsl(var(--muted))' }}
        >
          .exe
        </code>{' '}
        installer from our{' '}
        <a
          href={GITHUB_RELEASES}
          target="_blank"
          rel="noopener noreferrer"
          className="underline underline-offset-2"
          style={{ color: 'hsl(var(--primary))' }}
        >
          releases page
        </a>
        .
      </p>
      <pre
        className="p-4 rounded-lg overflow-x-auto text-sm mb-6 font-mono"
        style={{ backgroundColor: 'hsl(var(--muted))' }}
      >
        <code style={{ color: 'hsl(var(--foreground))' }}>
{`# Or using winget (coming soon)
winget install soul-player`}
        </code>
      </pre>

      {/* macOS */}
      <h3
        className="text-xl font-medium mb-3 mt-8"
        style={{ color: 'hsl(var(--foreground))' }}
      >
        macOS
      </h3>
      <p className="mb-3" style={{ color: 'hsl(var(--muted-foreground))' }}>
        Download the latest{' '}
        <code
          className="px-1.5 py-0.5 rounded text-sm"
          style={{ backgroundColor: 'hsl(var(--muted))' }}
        >
          .dmg
        </code>{' '}
        file from our{' '}
        <a
          href={GITHUB_RELEASES}
          target="_blank"
          rel="noopener noreferrer"
          className="underline underline-offset-2"
          style={{ color: 'hsl(var(--primary))' }}
        >
          releases page
        </a>
        .
      </p>
      <pre
        className="p-4 rounded-lg overflow-x-auto text-sm mb-6 font-mono"
        style={{ backgroundColor: 'hsl(var(--muted))' }}
      >
        <code style={{ color: 'hsl(var(--foreground))' }}>
{`# Or using Homebrew (coming soon)
brew install --cask soul-player`}
        </code>
      </pre>

      {/* Linux */}
      <h3
        className="text-xl font-medium mb-3 mt-8"
        style={{ color: 'hsl(var(--foreground))' }}
      >
        Linux
      </h3>
      <p className="mb-3" style={{ color: 'hsl(var(--muted-foreground))' }}>
        Download the latest{' '}
        <code
          className="px-1.5 py-0.5 rounded text-sm"
          style={{ backgroundColor: 'hsl(var(--muted))' }}
        >
          .AppImage
        </code>{' '}
        or{' '}
        <code
          className="px-1.5 py-0.5 rounded text-sm"
          style={{ backgroundColor: 'hsl(var(--muted))' }}
        >
          .deb
        </code>{' '}
        package from our{' '}
        <a
          href={GITHUB_RELEASES}
          target="_blank"
          rel="noopener noreferrer"
          className="underline underline-offset-2"
          style={{ color: 'hsl(var(--primary))' }}
        >
          releases page
        </a>
        .
      </p>
      <pre
        className="p-4 rounded-lg overflow-x-auto text-sm mb-6 font-mono"
        style={{ backgroundColor: 'hsl(var(--muted))' }}
      >
        <code style={{ color: 'hsl(var(--foreground))' }}>
{`# Debian/Ubuntu
sudo dpkg -i soul-player_*.deb

# Or run the AppImage directly
chmod +x Soul-Player-*.AppImage
./Soul-Player-*.AppImage`}
        </code>
      </pre>

      {/* Building from Source */}
      <h2
        className="text-2xl font-serif mb-4 mt-10 pb-2 border-b"
        style={{ color: 'hsl(var(--foreground))', borderColor: 'hsl(var(--border))' }}
      >
        Building from Source
      </h2>
      <p className="mb-4" style={{ color: 'hsl(var(--muted-foreground))' }}>
        Soul Player is built with Rust and TypeScript. To build from source:
      </p>

      <h3
        className="text-xl font-medium mb-3 mt-6"
        style={{ color: 'hsl(var(--foreground))' }}
      >
        Prerequisites
      </h3>
      <ul
        className="list-disc list-inside space-y-1 mb-4 ml-4"
        style={{ color: 'hsl(var(--muted-foreground))' }}
      >
        <li>
          <a
            href="https://rustup.rs/"
            target="_blank"
            rel="noopener noreferrer"
            className="underline underline-offset-2"
            style={{ color: 'hsl(var(--primary))' }}
          >
            Rust
          </a>{' '}
          (1.75 or later)
        </li>
        <li>
          <a
            href="https://nodejs.org/"
            target="_blank"
            rel="noopener noreferrer"
            className="underline underline-offset-2"
            style={{ color: 'hsl(var(--primary))' }}
          >
            Node.js
          </a>{' '}
          (20 or later)
        </li>
        <li>
          <a
            href="https://yarnpkg.com/"
            target="_blank"
            rel="noopener noreferrer"
            className="underline underline-offset-2"
            style={{ color: 'hsl(var(--primary))' }}
          >
            Yarn
          </a>{' '}
          (4.x via Corepack)
        </li>
      </ul>

      <h3
        className="text-xl font-medium mb-3 mt-6"
        style={{ color: 'hsl(var(--foreground))' }}
      >
        Build Steps
      </h3>
      <pre
        className="p-4 rounded-lg overflow-x-auto text-sm mb-6 font-mono"
        style={{ backgroundColor: 'hsl(var(--muted))' }}
      >
        <code style={{ color: 'hsl(var(--foreground))' }}>
{`# Clone the repository
git clone https://github.com/soulaudio/soul-player.git
cd soul-player

# Enable Yarn 4.x
corepack enable

# Install dependencies
yarn install

# Build the desktop app
yarn build:desktop`}
        </code>
      </pre>

      {/* System Requirements */}
      <h2
        className="text-2xl font-serif mb-4 mt-10 pb-2 border-b"
        style={{ color: 'hsl(var(--foreground))', borderColor: 'hsl(var(--border))' }}
      >
        System Requirements
      </h2>
      <div className="overflow-x-auto mb-6">
        <table className="w-full text-sm" style={{ color: 'hsl(var(--muted-foreground))' }}>
          <thead>
            <tr style={{ borderBottom: '1px solid hsl(var(--border))' }}>
              <th className="text-left py-2 pr-4 font-medium" style={{ color: 'hsl(var(--foreground))' }}>Component</th>
              <th className="text-left py-2 pr-4 font-medium" style={{ color: 'hsl(var(--foreground))' }}>Minimum</th>
              <th className="text-left py-2 font-medium" style={{ color: 'hsl(var(--foreground))' }}>Recommended</th>
            </tr>
          </thead>
          <tbody>
            <tr style={{ borderBottom: '1px solid hsl(var(--border))' }}>
              <td className="py-2 pr-4">OS</td>
              <td className="py-2 pr-4">Windows 10, macOS 11, Ubuntu 20.04</td>
              <td className="py-2">Latest versions</td>
            </tr>
            <tr style={{ borderBottom: '1px solid hsl(var(--border))' }}>
              <td className="py-2 pr-4">RAM</td>
              <td className="py-2 pr-4">2 GB</td>
              <td className="py-2">4 GB</td>
            </tr>
            <tr>
              <td className="py-2 pr-4">Storage</td>
              <td className="py-2 pr-4">100 MB</td>
              <td className="py-2">500 MB</td>
            </tr>
          </tbody>
        </table>
      </div>

      {/* Next Steps */}
      <h2
        className="text-2xl font-serif mb-4 mt-10 pb-2 border-b"
        style={{ color: 'hsl(var(--foreground))', borderColor: 'hsl(var(--border))' }}
      >
        Next Steps
      </h2>
      <p className="mb-4" style={{ color: 'hsl(var(--muted-foreground))' }}>
        After installation:
      </p>
      <ol
        className="list-decimal list-inside space-y-2 ml-4"
        style={{ color: 'hsl(var(--muted-foreground))' }}
      >
        <li>Launch Soul Player</li>
        <li>Click "Add Folder" to add your music library</li>
        <li>Wait for the initial scan to complete</li>
        <li>Start enjoying your music!</li>
      </ol>
    </>
  )
}
