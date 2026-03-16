import { createContext, useContext, useState, useEffect, useCallback, ReactNode } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import { debug } from '@soul-player/shared';
import { UpdateDialog } from '../components/UpdateDialog';

interface UpdateInfo {
  version: string;
  date?: string;
  body?: string;
}

interface UpdateSettingsContextValue {
  autoUpdate: boolean;
  silentUpdate: boolean;
  checking: boolean;
  onAutoUpdateChange: (enabled: boolean) => Promise<void>;
  onSilentUpdateChange: (enabled: boolean) => Promise<void>;
  checkForUpdates: () => Promise<void>;
}

const UpdateSettingsContext = createContext<UpdateSettingsContextValue>({
  autoUpdate: true,
  silentUpdate: false,
  checking: false,
  onAutoUpdateChange: async () => {},
  onSilentUpdateChange: async () => {},
  checkForUpdates: async () => {},
});

export function useUpdateSettings() {
  return useContext(UpdateSettingsContext);
}

export function UpdateSettingsProvider({ children }: { children: ReactNode }) {
  const { t } = useTranslation();
  const [autoUpdate, setAutoUpdate] = useState(true);
  const [silentUpdate, setSilentUpdate] = useState(false);
  const [checking, setChecking] = useState(false);
  const [showUpdateDialog, setShowUpdateDialog] = useState(false);
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [isInstalling, setIsInstalling] = useState(false);
  const [installProgress, setInstallProgress] = useState(0);
  const [installError, setInstallError] = useState<string | null>(null);

  useEffect(() => {
    const loadSettings = async () => {
      try {
        const autoUpdateSetting = await invoke<string | null>('get_user_setting', {
          key: 'app.auto_update_enabled',
        });
        if (autoUpdateSetting !== null) setAutoUpdate(JSON.parse(autoUpdateSetting));

        const silentUpdateSetting = await invoke<string | null>('get_user_setting', {
          key: 'app.auto_update_silent',
        });
        if (silentUpdateSetting !== null) setSilentUpdate(JSON.parse(silentUpdateSetting));
      } catch (error) {
        debug.error('Failed to load update settings:', error);
      }
    };
    loadSettings();
  }, []);

  useEffect(() => {
    const unlistenAvailable = listen<UpdateInfo>('update-available', (event) => {
      setUpdateInfo(event.payload);
      setShowUpdateDialog(true);
    });

    const unlistenProgress = listen<number>('update-progress', (event) => {
      setInstallProgress(event.payload);
    });

    return () => {
      unlistenAvailable.then((fn) => fn());
      unlistenProgress.then((fn) => fn());
    };
  }, []);

  const onAutoUpdateChange = useCallback(async (enabled: boolean) => {
    try {
      await invoke('set_user_setting', {
        key: 'app.auto_update_enabled',
        value: JSON.stringify(enabled),
      });
      setAutoUpdate(enabled);
    } catch (error) {
      debug.error('Failed to save auto-update setting:', error);
    }
  }, []);

  const onSilentUpdateChange = useCallback(async (enabled: boolean) => {
    try {
      await invoke('set_user_setting', {
        key: 'app.auto_update_silent',
        value: JSON.stringify(enabled),
      });
      setSilentUpdate(enabled);
    } catch (error) {
      debug.error('Failed to save silent update setting:', error);
    }
  }, []);

  const checkForUpdates = useCallback(async () => {
    setChecking(true);
    try {
      const update = await invoke<UpdateInfo | null>('check_for_updates');
      if (update) {
        setUpdateInfo(update);
        setShowUpdateDialog(true);
      } else {
        toast.success(t('settings.upToDate'));
      }
    } catch (error) {
      debug.error('Failed to check for updates:', error);
      toast.error(t('settings.checkFailed'));
    } finally {
      setChecking(false);
    }
  }, [t]);

  const handleInstallUpdate = useCallback(async () => {
    if (isInstalling) return;
    setIsInstalling(true);
    setInstallProgress(0);
    setInstallError(null);
    try {
      await invoke('install_update');
      toast.success(t('settings.updateInstalledRestarting'));
      setShowUpdateDialog(false);
    } catch (error) {
      const message = typeof error === 'string' ? error : String(error);
      debug.error('Failed to install update:', message);
      setInstallError(message);
      setIsInstalling(false);
      setInstallProgress(0);
    }
  }, [t, isInstalling]);

  const handleCloseUpdateDialog = useCallback(() => {
    if (!isInstalling) {
      setShowUpdateDialog(false);
      setUpdateInfo(null);
      setInstallProgress(0);
      setInstallError(null);
    }
  }, [isInstalling]);

  return (
    <UpdateSettingsContext.Provider
      value={{ autoUpdate, silentUpdate, checking, onAutoUpdateChange, onSilentUpdateChange, checkForUpdates }}
    >
      {children}
      <UpdateDialog
        open={showUpdateDialog}
        onClose={handleCloseUpdateDialog}
        onInstall={handleInstallUpdate}
        updateInfo={updateInfo}
        isInstalling={isInstalling}
        progress={installProgress}
        installError={installError}
      />
    </UpdateSettingsContext.Provider>
  );
}
