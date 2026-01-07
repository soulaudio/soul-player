import Link from 'next/link'
import { Github } from 'lucide-react'

export function Footer() {
  return (
    <footer className="border-t border-zinc-800 bg-black">
      <div className="container mx-auto px-6 py-12">
        <div className="grid grid-cols-1 md:grid-cols-4 gap-8">
          <div>
            <h3 className="font-bold text-lg mb-4">Soul Player</h3>
            <p className="text-zinc-400 text-sm">
              Local-first music player for the modern age
            </p>
          </div>

          <div>
            <h4 className="font-semibold mb-4 text-sm">Product</h4>
            <ul className="space-y-2 text-sm text-zinc-400">
              <li>
                <Link href="/#demo" className="hover:text-white transition-colors">
                  Demo
                </Link>
              </li>
              <li>
                <Link href="/docs" className="hover:text-white transition-colors">
                  Documentation
                </Link>
              </li>
              <li>
                <a href="#" className="hover:text-white transition-colors">
                  Download
                </a>
              </li>
            </ul>
          </div>

          <div>
            <h4 className="font-semibold mb-4 text-sm">Resources</h4>
            <ul className="space-y-2 text-sm text-zinc-400">
              <li>
                <Link href="/docs" className="hover:text-white transition-colors">
                  Getting Started
                </Link>
              </li>
              <li>
                <a href="#" className="hover:text-white transition-colors">
                  GitHub
                </a>
              </li>
              <li>
                <a href="#" className="hover:text-white transition-colors">
                  Community
                </a>
              </li>
            </ul>
          </div>

          <div>
            <h4 className="font-semibold mb-4 text-sm">Connect</h4>
            <div className="flex gap-4">
              <a
                href="https://github.com/soulaudio/soul-player"
                target="_blank"
                rel="noopener noreferrer"
                className="hover:text-violet-400 transition-colors"
              >
                <Github className="w-5 h-5" />
              </a>
            </div>
          </div>
        </div>

        <div className="border-t border-zinc-800 mt-12 pt-8 text-center text-sm text-zinc-400">
          <p>© {new Date().getFullYear()} Soul Player. Your music, your way.</p>
        </div>
      </div>
    </footer>
  )
}
