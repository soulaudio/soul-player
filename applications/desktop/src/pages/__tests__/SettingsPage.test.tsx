/**
 * Tests for SettingsPage update functionality
 * Tests update settings, check for updates, and update dialog integration
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { SettingsPage } from '../SettingsPage';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { SettingsProvider } from '../../contexts/SettingsContext';
import { I18nextProvider } from 'react-i18next';
import i18n from 'i18next';

// Mock Tauri APIs
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(),
}));

// Mock toast
vi.mock('sonner', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

// Initialize i18n for tests
i18n.init({
  lng: 'en-US',
  resources: {
    'en-US': {
      translation: {
        settings: {
          general: 'General',
          updates: 'Updates',
          autoUpdate: 'Automatically check for updates',
          silentUpdate: 'Install updates silently',
          checkNow: 'Check Now',
          checking: 'Checking...',
          upToDate: 'You\'re on the latest version!',
          checkFailed: 'Failed to check for updates',
        },
        updateDialog: {
          title: 'Update Available',
          later: 'Later',
          installNow: 'Install Now',
        },
      },
    },
  },
});

const renderSettingsPage = () => {
  return render(
    <I18nextProvider i18n={i18n}>
      <SettingsProvider>
        <SettingsPage />
      </SettingsProvider>
    </I18nextProvider>
  );
};

describe('SettingsPage - Update Functionality', () => {
  beforeEach(() => {
    vi.clearAllMocks();

    // Default mock implementations
    (invoke as unknown as ReturnType<typeof vi.fn>).mockImplementation((cmd: string) => {
      if (cmd === 'get_user_setting') {
        return Promise.resolve(null);
      }
      return Promise.resolve(undefined);
    });

    (listen as unknown as ReturnType<typeof vi.fn>).mockImplementation(() => {
      return Promise.resolve(() => {});
    });
  });

  describe('update settings loading', () => {
    it('should load auto-update setting on mount', async () => {
      (invoke as unknown as ReturnType<typeof vi.fn>).mockImplementation((cmd: string, args?: any) => {
        if (cmd === 'get_user_setting' && args?.key === 'app.auto_update_enabled') {
          return Promise.resolve(JSON.stringify(true));
        }
        return Promise.resolve(null);
      });

      renderSettingsPage();

      await waitFor(() => {
        expect(invoke).toHaveBeenCalledWith('get_user_setting', {
          key: 'app.auto_update_enabled',
        });
      });
    });

    it('should load silent update setting on mount', async () => {
      (invoke as unknown as ReturnType<typeof vi.fn>).mockImplementation((cmd: string, args?: any) => {
        if (cmd === 'get_user_setting' && args?.key === 'app.auto_update_silent') {
          return Promise.resolve(JSON.stringify(false));
        }
        return Promise.resolve(null);
      });

      renderSettingsPage();

      await waitFor(() => {
        expect(invoke).toHaveBeenCalledWith('get_user_setting', {
          key: 'app.auto_update_silent',
        });
      });
    });

    it('should default to auto-update enabled when no setting exists', async () => {
      (invoke as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(null);

      renderSettingsPage();

      await waitFor(() => {
        const checkbox = screen.getByLabelText('Automatically check for updates') as HTMLInputElement;
        expect(checkbox.checked).toBe(true);
      });
    });

    it('should default to silent update disabled when no setting exists', async () => {
      (invoke as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(null);

      renderSettingsPage();

      await waitFor(() => {
        const checkbox = screen.getByLabelText('Install updates silently') as HTMLInputElement;
        expect(checkbox.checked).toBe(false);
      });
    });
  });

  describe('auto-update toggle', () => {
    it('should save auto-update setting when toggled', async () => {
      renderSettingsPage();

      const checkbox = await screen.findByLabelText('Automatically check for updates');
      fireEvent.click(checkbox);

      await waitFor(() => {
        expect(invoke).toHaveBeenCalledWith('set_user_setting', {
          key: 'app.auto_update_enabled',
          value: JSON.stringify(false),
        });
      });
    });

    it('should update checkbox state after toggle', async () => {
      let autoUpdateEnabled = true;

      (invoke as unknown as ReturnType<typeof vi.fn>).mockImplementation((cmd: string, args?: any) => {
        if (cmd === 'get_user_setting' && args?.key === 'app.auto_update_enabled') {
          return Promise.resolve(JSON.stringify(autoUpdateEnabled));
        }
        if (cmd === 'set_user_setting' && args?.key === 'app.auto_update_enabled') {
          autoUpdateEnabled = JSON.parse(args.value);
          return Promise.resolve(undefined);
        }
        return Promise.resolve(null);
      });

      renderSettingsPage();

      const checkbox = await screen.findByLabelText('Automatically check for updates') as HTMLInputElement;
      expect(checkbox.checked).toBe(true);

      fireEvent.click(checkbox);

      await waitFor(() => {
        expect(checkbox.checked).toBe(false);
      });
    });

    it('should handle save errors gracefully', async () => {
      const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

      (invoke as unknown as ReturnType<typeof vi.fn>).mockImplementation((cmd: string) => {
        if (cmd === 'set_user_setting') {
          return Promise.reject(new Error('Failed to save'));
        }
        return Promise.resolve(null);
      });

      renderSettingsPage();

      const checkbox = await screen.findByLabelText('Automatically check for updates');
      fireEvent.click(checkbox);

      await waitFor(() => {
        expect(consoleErrorSpy).toHaveBeenCalled();
      });

      consoleErrorSpy.mockRestore();
    });
  });

  describe('silent update toggle', () => {
    it('should save silent update setting when toggled', async () => {
      renderSettingsPage();

      const checkbox = await screen.findByLabelText('Install updates silently');
      fireEvent.click(checkbox);

      await waitFor(() => {
        expect(invoke).toHaveBeenCalledWith('set_user_setting', {
          key: 'app.auto_update_silent',
          value: JSON.stringify(true),
        });
      });
    });

    it('should be disabled when auto-update is disabled', async () => {
      (invoke as unknown as ReturnType<typeof vi.fn>).mockImplementation((cmd: string, args?: any) => {
        if (cmd === 'get_user_setting' && args?.key === 'app.auto_update_enabled') {
          return Promise.resolve(JSON.stringify(false));
        }
        return Promise.resolve(null);
      });

      renderSettingsPage();

      await waitFor(() => {
        const checkbox = screen.getByLabelText('Install updates silently') as HTMLInputElement;
        expect(checkbox.disabled).toBe(true);
      });
    });

    it('should be enabled when auto-update is enabled', async () => {
      (invoke as unknown as ReturnType<typeof vi.fn>).mockImplementation((cmd: string, args?: any) => {
        if (cmd === 'get_user_setting' && args?.key === 'app.auto_update_enabled') {
          return Promise.resolve(JSON.stringify(true));
        }
        return Promise.resolve(null);
      });

      renderSettingsPage();

      await waitFor(() => {
        const checkbox = screen.getByLabelText('Install updates silently') as HTMLInputElement;
        expect(checkbox.disabled).toBe(false);
      });
    });
  });

  describe('check for updates button', () => {
    it('should render check for updates button', async () => {
      renderSettingsPage();

      await waitFor(() => {
        expect(screen.getByText('Check Now')).toBeInTheDocument();
      });
    });

    it('should call check_for_updates when clicked', async () => {
      (invoke as unknown as ReturnType<typeof vi.fn>).mockImplementation((cmd: string) => {
        if (cmd === 'check_for_updates') {
          return Promise.resolve(null);
        }
        return Promise.resolve(null);
      });

      renderSettingsPage();

      const button = await screen.findByText('Check Now');
      fireEvent.click(button);

      await waitFor(() => {
        expect(invoke).toHaveBeenCalledWith('check_for_updates');
      });
    });

    it('should show "Checking..." text while checking', async () => {
      let resolveCheck: () => void;
      const checkPromise = new Promise<null>((resolve) => {
        resolveCheck = () => resolve(null);
      });

      (invoke as unknown as ReturnType<typeof vi.fn>).mockImplementation((cmd: string) => {
        if (cmd === 'check_for_updates') {
          return checkPromise;
        }
        return Promise.resolve(null);
      });

      renderSettingsPage();

      const button = await screen.findByText('Check Now');
      fireEvent.click(button);

      await waitFor(() => {
        expect(screen.getByText('Checking...')).toBeInTheDocument();
      });

      resolveCheck!();

      await waitFor(() => {
        expect(screen.getByText('Check Now')).toBeInTheDocument();
      });
    });

    it('should disable button while checking', async () => {
      let resolveCheck: () => void;
      const checkPromise = new Promise<null>((resolve) => {
        resolveCheck = () => resolve(null);
      });

      (invoke as unknown as ReturnType<typeof vi.fn>).mockImplementation((cmd: string) => {
        if (cmd === 'check_for_updates') {
          return checkPromise;
        }
        return Promise.resolve(null);
      });

      renderSettingsPage();

      const button = await screen.findByText('Check Now');
      fireEvent.click(button);

      await waitFor(() => {
        const checkingButton = screen.getByText('Checking...') as HTMLButtonElement;
        expect(checkingButton.disabled).toBe(true);
      });

      resolveCheck!();
    });

    it('should show success toast when up to date', async () => {
      const { toast } = await import('sonner');

      (invoke as unknown as ReturnType<typeof vi.fn>).mockImplementation((cmd: string) => {
        if (cmd === 'check_for_updates') {
          return Promise.resolve(null);
        }
        return Promise.resolve(null);
      });

      renderSettingsPage();

      const button = await screen.findByText('Check Now');
      fireEvent.click(button);

      await waitFor(() => {
        expect(toast.success).toHaveBeenCalledWith('You\'re on the latest version!');
      });
    });

    it('should show update dialog when update is available', async () => {
      const updateInfo = {
        version: '1.5.0',
        date: '2024-01-15',
        body: 'Bug fixes',
      };

      (invoke as unknown as ReturnType<typeof vi.fn>).mockImplementation((cmd: string) => {
        if (cmd === 'check_for_updates') {
          return Promise.resolve(updateInfo);
        }
        return Promise.resolve(null);
      });

      renderSettingsPage();

      const button = await screen.findByText('Check Now');
      fireEvent.click(button);

      await waitFor(() => {
        expect(screen.getByText('Update Available')).toBeInTheDocument();
        expect(screen.getByText('v1.5.0')).toBeInTheDocument();
      });
    });

    it('should show error toast when check fails', async () => {
      const { toast } = await import('sonner');
      const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

      (invoke as unknown as ReturnType<typeof vi.fn>).mockImplementation((cmd: string) => {
        if (cmd === 'check_for_updates') {
          return Promise.reject(new Error('Network error'));
        }
        return Promise.resolve(null);
      });

      renderSettingsPage();

      const button = await screen.findByText('Check Now');
      fireEvent.click(button);

      await waitFor(() => {
        expect(toast.error).toHaveBeenCalledWith('Failed to check for updates');
      });

      consoleErrorSpy.mockRestore();
    });
  });

  describe('update dialog integration', () => {
    it('should listen for update-available events', () => {
      renderSettingsPage();

      expect(listen).toHaveBeenCalledWith('update-available', expect.any(Function));
    });

    it('should show dialog when update-available event fires', async () => {
      let updateHandler: ((event: any) => void) | undefined;

      (listen as unknown as ReturnType<typeof vi.fn>).mockImplementation((eventName: string, handler: any) => {
        if (eventName === 'update-available') {
          updateHandler = handler;
        }
        return Promise.resolve(() => {});
      });

      renderSettingsPage();

      await waitFor(() => {
        expect(updateHandler).toBeDefined();
      });

      const updateInfo = {
        version: '1.5.0',
        date: '2024-01-15',
        body: 'Bug fixes',
      };

      updateHandler!({ payload: updateInfo });

      await waitFor(() => {
        expect(screen.getByText('Update Available')).toBeInTheDocument();
        expect(screen.getByText('v1.5.0')).toBeInTheDocument();
      });
    });

    it('should listen for update-progress events', () => {
      renderSettingsPage();

      expect(listen).toHaveBeenCalledWith('update-progress', expect.any(Function));
    });

    it('should update progress when update-progress event fires', async () => {
      let progressHandler: ((event: any) => void) | undefined;
      let updateHandler: ((event: any) => void) | undefined;

      (listen as unknown as ReturnType<typeof vi.fn>).mockImplementation((eventName: string, handler: any) => {
        if (eventName === 'update-available') {
          updateHandler = handler;
        } else if (eventName === 'update-progress') {
          progressHandler = handler;
        }
        return Promise.resolve(() => {});
      });

      renderSettingsPage();

      await waitFor(() => {
        expect(updateHandler).toBeDefined();
        expect(progressHandler).toBeDefined();
      });

      // Trigger update available
      updateHandler!({
        payload: {
          version: '1.5.0',
          body: 'Bug fixes',
        },
      });

      await waitFor(() => {
        expect(screen.getByText('Update Available')).toBeInTheDocument();
      });

      // Start installation
      const installButton = screen.getByText('Install Now');
      fireEvent.click(installButton);

      // Update progress
      progressHandler!({ payload: 50 });

      await waitFor(() => {
        expect(screen.getByText('50%')).toBeInTheDocument();
      });
    });

    it('should call install_update when install button is clicked', async () => {
      let updateHandler: ((event: any) => void) | undefined;

      (listen as unknown as ReturnType<typeof vi.fn>).mockImplementation((eventName: string, handler: any) => {
        if (eventName === 'update-available') {
          updateHandler = handler;
        }
        return Promise.resolve(() => {});
      });

      (invoke as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(undefined);

      renderSettingsPage();

      await waitFor(() => {
        expect(updateHandler).toBeDefined();
      });

      updateHandler!({
        payload: {
          version: '1.5.0',
          body: 'Bug fixes',
        },
      });

      await waitFor(() => {
        expect(screen.getByText('Update Available')).toBeInTheDocument();
      });

      const installButton = screen.getByText('Install Now');
      fireEvent.click(installButton);

      await waitFor(() => {
        expect(invoke).toHaveBeenCalledWith('install_update');
      });
    });

    it('should close dialog when close button is clicked', async () => {
      let updateHandler: ((event: any) => void) | undefined;

      (listen as unknown as ReturnType<typeof vi.fn>).mockImplementation((eventName: string, handler: any) => {
        if (eventName === 'update-available') {
          updateHandler = handler;
        }
        return Promise.resolve(() => {});
      });

      renderSettingsPage();

      await waitFor(() => {
        expect(updateHandler).toBeDefined();
      });

      updateHandler!({
        payload: {
          version: '1.5.0',
          body: 'Bug fixes',
        },
      });

      await waitFor(() => {
        expect(screen.getByText('Update Available')).toBeInTheDocument();
      });

      const laterButton = screen.getByText('Later');
      fireEvent.click(laterButton);

      await waitFor(() => {
        expect(screen.queryByText('Update Available')).not.toBeInTheDocument();
      });
    });

    it('should cleanup event listeners on unmount', () => {
      const unlistenUpdate = vi.fn();
      const unlistenProgress = vi.fn();

      (listen as unknown as ReturnType<typeof vi.fn>).mockImplementation((eventName: string) => {
        if (eventName === 'update-available') {
          return Promise.resolve(unlistenUpdate);
        } else if (eventName === 'update-progress') {
          return Promise.resolve(unlistenProgress);
        }
        return Promise.resolve(() => {});
      });

      const { unmount } = renderSettingsPage();

      unmount();

      expect(unlistenUpdate).toHaveBeenCalled();
      expect(unlistenProgress).toHaveBeenCalled();
    });
  });
});
