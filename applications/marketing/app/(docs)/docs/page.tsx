import Link from 'next/link'
import { GITHUB_ISSUES, GITHUB_DISCUSSIONS } from '@/constants/links'

export const metadata = {
  title: 'Introduction',
  description: 'Welcome to Soul Player documentation. Learn about features and getting started.',
}

export const dynamic = 'force-static'

export default function DocsPage() {
  return (
    <>
      <h1
        className="text-4xl font-serif mb-6"
        style={{ color: 'hsl(var(--foreground))' }}
      >
        Soul Player Documentation
      </h1>

      <p
        className="text-lg mb-8 leading-relaxed"
        style={{ color: 'hsl(var(--muted-foreground))' }}
      >
        Welcome to Soul Player, a free and open-source music player that puts you in control of your music library.
      </p>

      <h2
        className="text-2xl font-serif mb-4 mt-10 pb-2 border-b"
        style={{ color: 'hsl(var(--foreground))', borderColor: 'hsl(var(--border))' }}
      >
        Quick Start
      </h2>

      <p className="mb-4" style={{ color: 'hsl(var(--muted-foreground))' }}>
        Get started with Soul Player in minutes:
      </p>

      <ol className="list-decimal list-inside space-y-2 mb-8 ml-4" style={{ color: 'hsl(var(--muted-foreground))' }}>
        <li>
          <Link
            href="/docs/installation"
            className="underline underline-offset-2"
            style={{ color: 'hsl(var(--primary))' }}
          >
            Install Soul Player
          </Link>
        </li>
        <li>Add your music folders</li>
        <li>Start listening</li>
      </ol>

      <h2
        className="text-2xl font-serif mb-4 mt-10 pb-2 border-b"
        style={{ color: 'hsl(var(--foreground))', borderColor: 'hsl(var(--border))' }}
      >
        Features
      </h2>

      <ul className="space-y-3 mb-8 ml-4" style={{ color: 'hsl(var(--muted-foreground))' }}>
        <li>
          <strong style={{ color: 'hsl(var(--foreground))' }}>Local-first</strong> - Your music stays on your device
        </li>
        <li>
          <strong style={{ color: 'hsl(var(--foreground))' }}>Multi-platform</strong> - Windows, macOS, and Linux support
        </li>
        <li>
          <strong style={{ color: 'hsl(var(--foreground))' }}>High-quality audio</strong> - Support for lossless formats (FLAC, ALAC, WAV)
        </li>
        <li>
          <strong style={{ color: 'hsl(var(--foreground))' }}>Beautiful UI</strong> - Modern, themeable interface
        </li>
        <li>
          <strong style={{ color: 'hsl(var(--foreground))' }}>Self-hosted streaming</strong> - Optional multi-user server mode
        </li>
      </ul>

      <h2
        className="text-2xl font-serif mb-4 mt-10 pb-2 border-b"
        style={{ color: 'hsl(var(--foreground))', borderColor: 'hsl(var(--border))' }}
      >
        Why Soul Player?
      </h2>

      <p className="mb-4" style={{ color: 'hsl(var(--muted-foreground))' }}>
        Unlike streaming services, Soul Player gives you complete control:
      </p>

      <div className="overflow-x-auto mb-8">
        <table className="w-full text-sm" style={{ color: 'hsl(var(--muted-foreground))' }}>
          <thead>
            <tr style={{ borderBottom: '1px solid hsl(var(--border))' }}>
              <th className="text-left py-2 pr-4 font-medium" style={{ color: 'hsl(var(--foreground))' }}>Feature</th>
              <th className="text-left py-2 pr-4 font-medium" style={{ color: 'hsl(var(--foreground))' }}>Soul Player</th>
              <th className="text-left py-2 font-medium" style={{ color: 'hsl(var(--foreground))' }}>Streaming Services</th>
            </tr>
          </thead>
          <tbody>
            <tr style={{ borderBottom: '1px solid hsl(var(--border))' }}>
              <td className="py-2 pr-4">Own your music</td>
              <td className="py-2 pr-4">Yes</td>
              <td className="py-2">No</td>
            </tr>
            <tr style={{ borderBottom: '1px solid hsl(var(--border))' }}>
              <td className="py-2 pr-4">Works offline</td>
              <td className="py-2 pr-4">Yes</td>
              <td className="py-2">Limited</td>
            </tr>
            <tr style={{ borderBottom: '1px solid hsl(var(--border))' }}>
              <td className="py-2 pr-4">No subscription</td>
              <td className="py-2 pr-4">Yes</td>
              <td className="py-2">No</td>
            </tr>
            <tr style={{ borderBottom: '1px solid hsl(var(--border))' }}>
              <td className="py-2 pr-4">Privacy</td>
              <td className="py-2 pr-4">Yes</td>
              <td className="py-2">No</td>
            </tr>
            <tr>
              <td className="py-2 pr-4">Lossless quality</td>
              <td className="py-2 pr-4">Yes</td>
              <td className="py-2">Varies</td>
            </tr>
          </tbody>
        </table>
      </div>

      <h2
        className="text-2xl font-serif mb-4 mt-10 pb-2 border-b"
        style={{ color: 'hsl(var(--foreground))', borderColor: 'hsl(var(--border))' }}
      >
        Getting Help
      </h2>

      <ul className="space-y-2 ml-4" style={{ color: 'hsl(var(--muted-foreground))' }}>
        <li>
          <a
            href={GITHUB_ISSUES}
            target="_blank"
            rel="noopener noreferrer"
            className="underline underline-offset-2"
            style={{ color: 'hsl(var(--primary))' }}
          >
            GitHub Issues
          </a>{' '}
          - Report bugs and request features
        </li>
        <li>
          <a
            href={GITHUB_DISCUSSIONS}
            target="_blank"
            rel="noopener noreferrer"
            className="underline underline-offset-2"
            style={{ color: 'hsl(var(--primary))' }}
          >
            Discussions
          </a>{' '}
          - Ask questions and share ideas
        </li>
      </ul>
    </>
  )
}
