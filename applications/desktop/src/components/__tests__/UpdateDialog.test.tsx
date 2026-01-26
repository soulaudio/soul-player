/**
 * Comprehensive tests for UpdateDialog component
 * Tests update UI rendering, installation methods, progress tracking, and user interactions
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { UpdateDialog } from '../UpdateDialog';
import { invoke } from '@tauri-apps/api/core';

// Mock Tauri invoke
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

// Mock clipboard API
const mockClipboard = {
  writeText: vi.fn().mockResolvedValue(undefined),
};

Object.assign(navigator, {
  clipboard: mockClipboard,
});

interface UpdateInfo {
  version: string;
  date?: string;
  body?: string;
}

interface InstallationInfo {
  method: {
    type: 'appimage' | 'deb' | 'rpm' | 'flatpak' | 'snap' | 'aur' | 'unknown';
  };
  update_command: string | null;
  supports_auto_update: boolean;
}

const mockUpdateInfo: UpdateInfo = {
  version: '1.5.0',
  date: '2024-01-15',
  body: 'Bug fixes and performance improvements',
};

const mockAppImageInstallation: InstallationInfo = {
  method: { type: 'appimage' },
  update_command: null,
  supports_auto_update: true,
};

const mockDebInstallation: InstallationInfo = {
  method: { type: 'deb' },
  update_command: 'sudo apt update && sudo apt upgrade soul-player',
  supports_auto_update: false,
};

const mockFlatpakInstallation: InstallationInfo = {
  method: { type: 'flatpak' },
  update_command: 'flatpak update io.github.soulaudio.SoulPlayer',
  supports_auto_update: false,
};

describe('UpdateDialog Component', () => {
  const mockOnClose = vi.fn();
  const mockOnInstall = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    mockClipboard.writeText.mockClear();
  });

  afterEach(() => {
    vi.clearAllTimers();
  });

  describe('basic rendering', () => {
    it('should not render when closed', () => {
      const { container } = render(
        <UpdateDialog
          open={false}
          onClose={mockOnClose}
          onInstall={mockOnInstall}
          updateInfo={mockUpdateInfo}
        />
      );

      expect(container.firstChild).toBeNull();
    });

    it('should not render when updateInfo is null', () => {
      const { container } = render(
        <UpdateDialog
          open={true}
          onClose={mockOnClose}
          onInstall={mockOnInstall}
          updateInfo={null}
        />
      );

      expect(container.firstChild).toBeNull();
    });

    it('should render dialog when open with updateInfo', () => {
      (invoke as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockAppImageInstallation);

      render(
        <UpdateDialog
          open={true}
          onClose={mockOnClose}
          onInstall={mockOnInstall}
          updateInfo={mockUpdateInfo}
        />
      );

      expect(screen.getByText('Update Available')).toBeInTheDocument();
      expect(screen.getByText('v1.5.0')).toBeInTheDocument();
    });

    it('should display version number', () => {
      (invoke as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockAppImageInstallation);

      render(
        <UpdateDialog
          open={true}
          onClose={mockOnClose}
          onInstall={mockOnInstall}
          updateInfo={mockUpdateInfo}
        />
      );

      expect(screen.getByText('v1.5.0')).toBeInTheDocument();
    });

    it('should display release notes', () => {
      (invoke as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockAppImageInstallation);

      render(
        <UpdateDialog
          open={true}
          onClose={mockOnClose}
          onInstall={mockOnInstall}
          updateInfo={mockUpdateInfo}
        />
      );

      expect(screen.getByText('What\'s New')).toBeInTheDocument();
      expect(screen.getByText('Bug fixes and performance improvements')).toBeInTheDocument();
    });

    it('should not display release notes section when body is empty', () => {
      (invoke as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockAppImageInstallation);

      render(
        <UpdateDialog
          open={true}
          onClose={mockOnClose}
          onInstall={mockOnInstall}
          updateInfo={{ version: '1.5.0' }}
        />
      );

      expect(screen.queryByText('What\'s New')).not.toBeInTheDocument();
    });
  });

  describe('installation info fetching', () => {
    it('should fetch installation info when dialog opens', async () => {
      (invoke as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockAppImageInstallation);

      render(
        <UpdateDialog
          open={true}
          onClose={mockOnClose}
          onInstall={mockOnInstall}
          updateInfo={mockUpdateInfo}
        />
      );

      await waitFor(() => {
        expect(invoke).toHaveBeenCalledWith('get_installation_info');
      });
    });

    it('should not fetch installation info when dialog is closed', () => {
      render(
        <UpdateDialog
          open={false}
          onClose={mockOnClose}
          onInstall={mockOnInstall}
          updateInfo={mockUpdateInfo}
        />
      );

      expect(invoke).not.toHaveBeenCalled();
    });

    it('should handle installation info fetch errors gracefully', async () => {
      const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
      (invoke as unknown as ReturnType<typeof vi.fn>).mockRejectedValue(new Error('Failed to fetch'));

      render(
        <UpdateDialog
          open={true}
          onClose={mockOnClose}
          onInstall={mockOnInstall}
          updateInfo={mockUpdateInfo}
        />
      );

      await waitFor(() => {
        expect(consoleErrorSpy).toHaveBeenCalled();
      });

      // Dialog should still render
      expect(screen.getByText('Update Available')).toBeInTheDocument();

      consoleErrorSpy.mockRestore();
    });
  });

  describe('AppImage installation (auto-update supported)', () => {
    it('should show "Install Now" button for AppImage', async () => {
      (invoke as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockAppImageInstallation);

      render(
        <UpdateDialog
          open={true}
          onClose={mockOnClose}
          onInstall={mockOnInstall}
          updateInfo={mockUpdateInfo}
        />
      );

      await waitFor(() => {
        expect(screen.getByText('Install Now')).toBeInTheDocument();
      });
    });

    it('should not show package manager instructions for AppImage', async () => {
      (invoke as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockAppImageInstallation);

      render(
        <UpdateDialog
          open={true}
          onClose={mockOnClose}
          onInstall={mockOnInstall}
          updateInfo={mockUpdateInfo}
        />
      );

      await waitFor(() => {
        expect(screen.queryByText('Package Manager Update Required')).not.toBeInTheDocument();
      });
    });

    it('should call onInstall when Install Now is clicked', async () => {
      (invoke as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockAppImageInstallation);

      render(
        <UpdateDialog
          open={true}
          onClose={mockOnClose}
          onInstall={mockOnInstall}
          updateInfo={mockUpdateInfo}
        />
      );

      const installButton = await screen.findByText('Install Now');
      fireEvent.click(installButton);

      expect(mockOnInstall).toHaveBeenCalledTimes(1);
    });

    it('should show progress bar when installing', async () => {
      (invoke as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockAppImageInstallation);

      render(
        <UpdateDialog
          open={true}
          onClose={mockOnClose}
          onInstall={mockOnInstall}
          updateInfo={mockUpdateInfo}
          isInstalling={true}
          progress={45}
        />
      );

      await waitFor(() => {
        expect(screen.getByText('Downloading update')).toBeInTheDocument();
        expect(screen.getByText('45%')).toBeInTheDocument();
      });
    });

    it('should disable install button when installing', async () => {
      (invoke as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockAppImageInstallation);

      render(
        <UpdateDialog
          open={true}
          onClose={mockOnClose}
          onInstall={mockOnInstall}
          updateInfo={mockUpdateInfo}
          isInstalling={true}
        />
      );

      const installButton = await screen.findByText('Installing...');
      expect(installButton).toBeDisabled();
    });

    it('should update progress bar width', async () => {
      (invoke as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockAppImageInstallation);

      const { rerender } = render(
        <UpdateDialog
          open={true}
          onClose={mockOnClose}
          onInstall={mockOnInstall}
          updateInfo={mockUpdateInfo}
          isInstalling={true}
          progress={25}
        />
      );

      await waitFor(() => {
        expect(screen.getByText('25%')).toBeInTheDocument();
      });

      rerender(
        <UpdateDialog
          open={true}
          onClose={mockOnClose}
          onInstall={mockOnInstall}
          updateInfo={mockUpdateInfo}
          isInstalling={true}
          progress={75}
        />
      );

      await waitFor(() => {
        expect(screen.getByText('75%')).toBeInTheDocument();
      });
    });
  });

  describe('package manager installations (no auto-update)', () => {
    it('should show package manager instructions for DEB', async () => {
      (invoke as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockDebInstallation);

      render(
        <UpdateDialog
          open={true}
          onClose={mockOnClose}
          onInstall={mockOnInstall}
          updateInfo={mockUpdateInfo}
        />
      );

      await waitFor(() => {
        expect(screen.getByText('Package Manager Update Required')).toBeInTheDocument();
        expect(screen.getByText('sudo apt update && sudo apt upgrade soul-player')).toBeInTheDocument();
      });
    });

    it('should show copy button for package manager command', async () => {
      (invoke as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockDebInstallation);

      render(
        <UpdateDialog
          open={true}
          onClose={mockOnClose}
          onInstall={mockOnInstall}
          updateInfo={mockUpdateInfo}
        />
      );

      await waitFor(() => {
        expect(screen.getByText('Copy')).toBeInTheDocument();
      });
    });

    it('should copy command to clipboard when copy button is clicked', async () => {
      vi.useFakeTimers();
      (invoke as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockDebInstallation);

      render(
        <UpdateDialog
          open={true}
          onClose={mockOnClose}
          onInstall={mockOnInstall}
          updateInfo={mockUpdateInfo}
        />
      );

      const copyButton = await screen.findByText('Copy');
      fireEvent.click(copyButton);

      expect(mockClipboard.writeText).toHaveBeenCalledWith('sudo apt update && sudo apt upgrade soul-player');

      await waitFor(() => {
        expect(screen.getByText('Copied!')).toBeInTheDocument();
      });

      // Should revert to "Copy" after 2 seconds
      vi.advanceTimersByTime(2000);

      await waitFor(() => {
        expect(screen.getByText('Copy')).toBeInTheDocument();
      });

      vi.useRealTimers();
    });

    it('should show "View Release" link instead of install button', async () => {
      (invoke as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockDebInstallation);

      render(
        <UpdateDialog
          open={true}
          onClose={mockOnClose}
          onInstall={mockOnInstall}
          updateInfo={mockUpdateInfo}
        />
      );

      await waitFor(() => {
        expect(screen.getByText('View Release')).toBeInTheDocument();
        expect(screen.queryByText('Install Now')).not.toBeInTheDocument();
      });
    });

    it('should link to GitHub release page', async () => {
      (invoke as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockFlatpakInstallation);

      render(
        <UpdateDialog
          open={true}
          onClose={mockOnClose}
          onInstall={mockOnInstall}
          updateInfo={mockUpdateInfo}
        />
      );

      const link = await screen.findByText('View Release');
      expect(link).toHaveAttribute('href', 'https://github.com/soulaudio/soul-player/releases/tag/v1.5.0');
      expect(link).toHaveAttribute('target', '_blank');
      expect(link).toHaveAttribute('rel', 'noopener noreferrer');
    });

    it('should show Flatpak update command', async () => {
      (invoke as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockFlatpakInstallation);

      render(
        <UpdateDialog
          open={true}
          onClose={mockOnClose}
          onInstall={mockOnInstall}
          updateInfo={mockUpdateInfo}
        />
      );

      await waitFor(() => {
        expect(screen.getByText('flatpak update io.github.soulaudio.SoulPlayer')).toBeInTheDocument();
      });
    });
  });

  describe('GitHub release link extraction', () => {
    it('should extract and show GitHub release link from release notes', async () => {
      (invoke as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockAppImageInstallation);

      const updateWithGitHubLink: UpdateInfo = {
        version: '1.5.0',
        body: 'New features:\n- Feature 1\n- Feature 2\n\nFull changelog: https://github.com/soulaudio/soul-player/releases/tag/v1.5.0',
      };

      render(
        <UpdateDialog
          open={true}
          onClose={mockOnClose}
          onInstall={mockOnInstall}
          updateInfo={updateWithGitHubLink}
        />
      );

      await waitFor(() => {
        const link = screen.getByText('View full release notes →');
        expect(link).toBeInTheDocument();
        expect(link).toHaveAttribute('href', 'https://github.com/soulaudio/soul-player/releases/tag/v1.5.0');
      });
    });

    it('should not show release link when not in notes', async () => {
      (invoke as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockAppImageInstallation);

      render(
        <UpdateDialog
          open={true}
          onClose={mockOnClose}
          onInstall={mockOnInstall}
          updateInfo={mockUpdateInfo}
        />
      );

      await waitFor(() => {
        expect(screen.queryByText('View full release notes →')).not.toBeInTheDocument();
      });
    });
  });

  describe('dialog interactions', () => {
    it('should call onClose when close button is clicked', async () => {
      (invoke as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockAppImageInstallation);

      render(
        <UpdateDialog
          open={true}
          onClose={mockOnClose}
          onInstall={mockOnInstall}
          updateInfo={mockUpdateInfo}
        />
      );

      const closeButton = screen.getAllByRole('button').find(btn => btn.querySelector('.lucide-x'));
      expect(closeButton).toBeDefined();
      fireEvent.click(closeButton!);

      expect(mockOnClose).toHaveBeenCalledTimes(1);
    });

    it('should call onClose when "Later" button is clicked', async () => {
      (invoke as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockAppImageInstallation);

      render(
        <UpdateDialog
          open={true}
          onClose={mockOnClose}
          onInstall={mockOnInstall}
          updateInfo={mockUpdateInfo}
        />
      );

      const laterButton = screen.getByText('Later');
      fireEvent.click(laterButton);

      expect(mockOnClose).toHaveBeenCalledTimes(1);
    });

    it('should call onClose when backdrop is clicked', async () => {
      (invoke as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockAppImageInstallation);

      const { container } = render(
        <UpdateDialog
          open={true}
          onClose={mockOnClose}
          onInstall={mockOnInstall}
          updateInfo={mockUpdateInfo}
        />
      );

      const backdrop = container.querySelector('.fixed.inset-0');
      expect(backdrop).toBeTruthy();
      fireEvent.click(backdrop!);

      expect(mockOnClose).toHaveBeenCalledTimes(1);
    });

    it('should call onClose when Escape key is pressed', async () => {
      (invoke as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockAppImageInstallation);

      const { container } = render(
        <UpdateDialog
          open={true}
          onClose={mockOnClose}
          onInstall={mockOnInstall}
          updateInfo={mockUpdateInfo}
        />
      );

      const backdrop = container.querySelector('.fixed.inset-0');
      fireEvent.keyDown(backdrop!, { key: 'Escape' });

      expect(mockOnClose).toHaveBeenCalledTimes(1);
    });

    it('should not close when clicking inside dialog', async () => {
      (invoke as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockAppImageInstallation);

      render(
        <UpdateDialog
          open={true}
          onClose={mockOnClose}
          onInstall={mockOnInstall}
          updateInfo={mockUpdateInfo}
        />
      );

      const dialogContent = screen.getByText('What\'s New');
      fireEvent.click(dialogContent);

      expect(mockOnClose).not.toHaveBeenCalled();
    });

    it('should not close on backdrop click when installing', async () => {
      (invoke as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockAppImageInstallation);

      const { container } = render(
        <UpdateDialog
          open={true}
          onClose={mockOnClose}
          onInstall={mockOnInstall}
          updateInfo={mockUpdateInfo}
          isInstalling={true}
        />
      );

      const backdrop = container.querySelector('.fixed.inset-0');
      fireEvent.click(backdrop!);

      expect(mockOnClose).not.toHaveBeenCalled();
    });

    it('should not close on Escape when installing', async () => {
      (invoke as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockAppImageInstallation);

      const { container } = render(
        <UpdateDialog
          open={true}
          onClose={mockOnClose}
          onInstall={mockOnInstall}
          updateInfo={mockUpdateInfo}
          isInstalling={true}
        />
      );

      const backdrop = container.querySelector('.fixed.inset-0');
      fireEvent.keyDown(backdrop!, { key: 'Escape' });

      expect(mockOnClose).not.toHaveBeenCalled();
    });

    it('should disable close button when installing', async () => {
      (invoke as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockAppImageInstallation);

      render(
        <UpdateDialog
          open={true}
          onClose={mockOnClose}
          onInstall={mockOnInstall}
          updateInfo={mockUpdateInfo}
          isInstalling={true}
        />
      );

      const closeButton = screen.getAllByRole('button').find(btn => btn.querySelector('.lucide-x'));
      expect(closeButton).toBeDisabled();
    });

    it('should disable "Later" button when installing', async () => {
      (invoke as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockAppImageInstallation);

      render(
        <UpdateDialog
          open={true}
          onClose={mockOnClose}
          onInstall={mockOnInstall}
          updateInfo={mockUpdateInfo}
          isInstalling={true}
        />
      );

      const laterButton = screen.getByText('Later');
      expect(laterButton).toBeDisabled();
    });
  });

  describe('edge cases', () => {
    it('should handle missing installation info gracefully', async () => {
      (invoke as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(null);

      render(
        <UpdateDialog
          open={true}
          onClose={mockOnClose}
          onInstall={mockOnInstall}
          updateInfo={mockUpdateInfo}
        />
      );

      // Should default to auto-update supported
      await waitFor(() => {
        expect(screen.getByText('Install Now')).toBeInTheDocument();
      });
    });

    it('should handle very long release notes', async () => {
      (invoke as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockAppImageInstallation);

      const longNotes = 'Line 1\n'.repeat(100);
      const updateWithLongNotes: UpdateInfo = {
        version: '1.5.0',
        body: longNotes,
      };

      const { container } = render(
        <UpdateDialog
          open={true}
          onClose={mockOnClose}
          onInstall={mockOnInstall}
          updateInfo={updateWithLongNotes}
        />
      );

      // Should have max-height and scroll
      const notesContainer = container.querySelector('.max-h-48');
      expect(notesContainer).toBeInTheDocument();
      expect(notesContainer?.classList.contains('overflow-y-auto')).toBe(true);
    });

    it('should preserve whitespace in release notes', async () => {
      (invoke as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockAppImageInstallation);

      const formattedNotes = 'Features:\n  - Feature 1\n  - Feature 2\n\nBug Fixes:\n  - Fix 1';
      const updateWithFormattedNotes: UpdateInfo = {
        version: '1.5.0',
        body: formattedNotes,
      };

      const { container } = render(
        <UpdateDialog
          open={true}
          onClose={mockOnClose}
          onInstall={mockOnInstall}
          updateInfo={updateWithFormattedNotes}
        />
      );

      const notesContainer = container.querySelector('.whitespace-pre-wrap');
      expect(notesContainer).toBeInTheDocument();
    });

    it('should handle progress values edge cases', async () => {
      (invoke as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(mockAppImageInstallation);

      const { rerender } = render(
        <UpdateDialog
          open={true}
          onClose={mockOnClose}
          onInstall={mockOnInstall}
          updateInfo={mockUpdateInfo}
          isInstalling={true}
          progress={0}
        />
      );

      await waitFor(() => {
        expect(screen.getByText('0%')).toBeInTheDocument();
      });

      rerender(
        <UpdateDialog
          open={true}
          onClose={mockOnClose}
          onInstall={mockOnInstall}
          updateInfo={mockUpdateInfo}
          isInstalling={true}
          progress={100}
        />
      );

      await waitFor(() => {
        expect(screen.getByText('100%')).toBeInTheDocument();
      });
    });
  });
});
