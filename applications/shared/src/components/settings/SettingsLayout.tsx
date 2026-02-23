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
      <aside className="w-14 flex-shrink-0 flex items-center justify-center">
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
