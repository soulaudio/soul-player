import type { MDXComponents } from 'mdx/types'
import Link from 'next/link'

export function useMDXComponents(components: MDXComponents): MDXComponents {
  return {
    h1: ({ children, id }) => (
      <h1
        id={id}
        className="text-4xl font-serif mb-6 mt-8 first:mt-0"
        style={{ color: 'hsl(var(--foreground))' }}
      >
        {children}
      </h1>
    ),
    h2: ({ children, id }) => (
      <h2
        id={id}
        className="text-2xl font-serif mb-4 mt-10 pb-2 border-b"
        style={{ color: 'hsl(var(--foreground))', borderColor: 'hsl(var(--border))' }}
      >
        {children}
      </h2>
    ),
    h3: ({ children, id }) => (
      <h3
        id={id}
        className="text-xl font-medium mb-3 mt-8"
        style={{ color: 'hsl(var(--foreground))' }}
      >
        {children}
      </h3>
    ),
    p: ({ children }) => (
      <p className="mb-4 leading-relaxed" style={{ color: 'hsl(var(--muted-foreground))' }}>
        {children}
      </p>
    ),
    strong: ({ children }) => (
      <strong style={{ color: 'hsl(var(--foreground))' }}>{children}</strong>
    ),
    a: ({ href, children }) => {
      const isExternal = href?.startsWith('http')
      if (isExternal) {
        return (
          <a
            href={href}
            target="_blank"
            rel="noopener noreferrer"
            className="underline underline-offset-2 transition-opacity hover:opacity-80"
            style={{ color: 'hsl(var(--primary))' }}
          >
            {children}
          </a>
        )
      }
      return (
        <Link
          href={href || '#'}
          className="underline underline-offset-2 transition-opacity hover:opacity-80"
          style={{ color: 'hsl(var(--primary))' }}
        >
          {children}
        </Link>
      )
    },
    ul: ({ children }) => (
      <ul className="list-disc list-inside space-y-2 mb-4 ml-4" style={{ color: 'hsl(var(--muted-foreground))' }}>
        {children}
      </ul>
    ),
    ol: ({ children }) => (
      <ol className="list-decimal list-inside space-y-2 mb-4 ml-4" style={{ color: 'hsl(var(--muted-foreground))' }}>
        {children}
      </ol>
    ),
    li: ({ children }) => <li className="leading-relaxed">{children}</li>,
    code: ({ children }) => (
      <code
        className="px-1.5 py-0.5 rounded text-sm font-mono"
        style={{ backgroundColor: 'hsl(var(--muted))', color: 'hsl(var(--foreground))' }}
      >
        {children}
      </code>
    ),
    pre: ({ children }) => (
      <pre
        className="p-4 rounded-lg overflow-x-auto text-sm mb-6 font-mono"
        style={{ backgroundColor: 'hsl(var(--muted))' }}
      >
        {children}
      </pre>
    ),
    table: ({ children }) => (
      <div className="overflow-x-auto mb-6">
        <table className="w-full text-sm" style={{ color: 'hsl(var(--muted-foreground))' }}>
          {children}
        </table>
      </div>
    ),
    th: ({ children }) => (
      <th className="text-left py-2 pr-4 font-medium" style={{ color: 'hsl(var(--foreground))' }}>
        {children}
      </th>
    ),
    td: ({ children }) => <td className="py-2 pr-4">{children}</td>,
    tr: ({ children }) => (
      <tr style={{ borderBottom: '1px solid hsl(var(--border))' }}>{children}</tr>
    ),
    blockquote: ({ children }) => (
      <blockquote
        className="border-l-4 pl-4 my-4 italic"
        style={{ borderColor: 'hsl(var(--primary))', color: 'hsl(var(--muted-foreground))' }}
      >
        {children}
      </blockquote>
    ),
    hr: () => <hr className="my-8" style={{ borderColor: 'hsl(var(--border))' }} />,
    ...components,
  }
}
