import { useState, useEffect, useCallback } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useTranslation } from 'react-i18next';
import { Minus, Square, X, Copy } from 'lucide-react';

/**
 * Window control buttons for frameless window.
 * Provides minimize, maximize/restore, and close buttons.
 * Positioned absolutely in the top right of the window.
 */
export function WindowControls() {
  const { t } = useTranslation();
  const [isMaximized, setIsMaximized] = useState(false);
  const appWindow = getCurrentWindow();

  useEffect(() => {
    const checkMaximized = async () => {
      try {
        const maximized = await appWindow.isMaximized();
        setIsMaximized(maximized);
      } catch (err) {
        console.error('Failed to check maximized state:', err);
      }
    };

    checkMaximized();

    const unlisten = appWindow.onResized(checkMaximized);

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [appWindow]);

  const handleMinimize = useCallback(async () => {
    try {
      await appWindow.minimize();
    } catch (err) {
      console.error('Failed to minimize window:', err);
    }
  }, [appWindow]);

  const handleMaximize = useCallback(async () => {
    try {
      await appWindow.toggleMaximize();
    } catch (err) {
      console.error('Failed to toggle maximize:', err);
    }
  }, [appWindow]);

  const handleClose = useCallback(async () => {
    try {
      await appWindow.close();
    } catch (err) {
      console.error('Failed to close window:', err);
    }
  }, [appWindow]);

  return (
    <div className="flex items-center">
      {/* Minimize */}
      <button
        onClick={handleMinimize}
        className="flex items-center justify-center w-11 h-8 hover:bg-foreground/[var(--hover-bg-opacity)] transition-colors duration-[var(--transition-duration)]"
        title={t('titleBar.minimize')}
        aria-label={t('titleBar.minimize')}
      >
        <Minus className="w-4 h-4 text-muted-foreground" />
      </button>

      {/* Maximize/Restore */}
      <button
        onClick={handleMaximize}
        className="flex items-center justify-center w-11 h-8 hover:bg-foreground/[var(--hover-bg-opacity)] transition-colors duration-[var(--transition-duration)]"
        title={isMaximized ? t('titleBar.restore') : t('titleBar.maximize')}
        aria-label={isMaximized ? t('titleBar.restore') : t('titleBar.maximize')}
      >
        {isMaximized ? (
          <Copy className="w-3 h-3 text-muted-foreground rotate-180" />
        ) : (
          <Square className="w-3 h-3 text-muted-foreground" />
        )}
      </button>

      {/* Close */}
      <button
        onClick={handleClose}
        className="flex items-center justify-center w-11 h-8 hover:bg-red-500 hover:text-white transition-colors"
        title={t('titleBar.close')}
        aria-label={t('titleBar.close')}
      >
        <X className="w-4 h-4" />
      </button>
    </div>
  );
}
