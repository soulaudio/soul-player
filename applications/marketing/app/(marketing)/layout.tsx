import { ThemeProvider } from '../providers'

export default function MarketingLayout({
  children,
}: {
  children: React.ReactNode
}) {
  return <ThemeProvider>{children}</ThemeProvider>
}
