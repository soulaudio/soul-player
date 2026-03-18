// Settings layout with sidebar navigation (desktop) or horizontal tabs (mobile)

import { ReactNode } from 'react';
import { SettingsSidebar } from './SettingsSidebar';
import { useSidebarState } from '../../contexts/SidebarStateContext';

interface SettingsLayoutProps {
  children: ReactNode;
}

export function SettingsLayout({ children }: SettingsLayoutProps) {
  const { isMobile } = useSidebarState();

  if (isMobile) {
    return (
      <div className="relative h-full">
        {/* Main content — full width, scrollable */}
        <main className="h-full overflow-y-auto">
          <div className="pr-6 pb-16">
            {children}
          </div>
        </main>

        {/* Floating settings tabs — centered just above the playback bar */}
        <div className="fixed bottom-[76px] left-0 right-0 z-40 flex justify-center pointer-events-none">
          <div className="pointer-events-auto">
            <SettingsSidebar horizontal />
          </div>
        </div>
      </div>
    );
  }

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
