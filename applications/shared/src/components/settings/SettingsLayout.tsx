// Settings layout with sidebar navigation

import { ReactNode } from 'react';
import { SettingsSidebar } from './SettingsSidebar';

interface SettingsLayoutProps {
  children: ReactNode;
}

export function SettingsLayout({ children }: SettingsLayoutProps) {
  return (
    <div className="flex h-full">
      {/* Sidebar */}
      <aside className="w-56 border-r border-border bg-card/30 flex-shrink-0">
        <SettingsSidebar />
      </aside>

      {/* Main content */}
      <main className="flex-1 overflow-y-auto">
        <div className="max-w-4xl mx-auto p-8">
          {children}
        </div>
      </main>
    </div>
  );
}
