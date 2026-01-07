/**
 * Desktop MainLayout - wrapper around shared MainLayout with ImportDialog
 */
import { ReactNode, useState } from 'react';
import { MainLayout as SharedMainLayout } from '@soul-player/shared';
import { ImportDialog } from '../components/ImportDialog';
import { useSettings } from '../contexts/SettingsContext';

interface MainLayoutProps {
  children: ReactNode;
}

export function MainLayout({ children }: MainLayoutProps) {
  const { showKeyboardShortcuts } = useSettings();
  const [showImportDialog, setShowImportDialog] = useState(false);

  return (
    <div className="h-screen">
      <SharedMainLayout
        onImport={() => setShowImportDialog(true)}
        showKeyboardShortcuts={showKeyboardShortcuts}
      >
        {children}
      </SharedMainLayout>

      {/* Desktop-specific ImportDialog */}
      <ImportDialog open={showImportDialog} onClose={() => setShowImportDialog(false)} />
    </div>
  );
}
