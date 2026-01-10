// Docs layout without ThemeProvider to enable proper static HTML output for search indexing
export default function DocsRootLayout({
  children,
}: {
  children: React.ReactNode
}) {
  return <>{children}</>
}
