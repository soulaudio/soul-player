'use client'

import { MemoryRouter } from 'react-router-dom'
import { MockThemeProvider, MockSettingsProvider } from './MockContexts'
import { LibraryPage } from './LibraryPage'
import { MainLayout } from './MainLayout'

/**
 * Demo version of the Soul Player app for marketing showcase
 * Uses mock contexts and data to avoid Tauri/backend dependencies
 * Fixed dimensions (1200x750) - will be scaled by DemoScaler
 */
export function DemoApp() {
  return (
    <div
      data-demo-container
      data-theme="dark"
      className="flex flex-col bg-background text-foreground"
      style={{ width: 1200, height: 750 }}
    >
      <MemoryRouter initialEntries={['/']}>
        <MockThemeProvider>
          <MockSettingsProvider>
            <MainLayout>
              <LibraryPage />
            </MainLayout>
          </MockSettingsProvider>
        </MockThemeProvider>
      </MemoryRouter>
    </div>
  )
}
