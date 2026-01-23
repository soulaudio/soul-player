import Link from 'next/link'

const docsNavigation = [
  { title: 'Introduction', href: '/docs' },
  { title: 'Installation', href: '/docs/installation' },
]

export function DocsSidebar() {
  return (
    <nav className="p-6 space-y-1">
      <h3 className="text-xs font-semibold uppercase tracking-wider mb-3 text-muted-foreground">
        Getting Started
      </h3>
      {docsNavigation.map((item) => (
        <Link
          key={item.href}
          href={item.href}
          className="sidebar-link block px-3 py-2 rounded-md text-sm transition-colors duration-[var(--transition-duration)] text-muted-foreground hover:bg-foreground/[var(--hover-bg-opacity)] hover:opacity-[var(--hover-text-opacity)] transition-opacity"
        >
          {item.title}
        </Link>
      ))}
    </nav>
  )
}
