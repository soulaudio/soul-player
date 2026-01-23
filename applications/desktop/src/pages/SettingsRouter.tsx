/**
 * Settings Router - handles nested settings routes with sidebar layout
 */

import { Routes, Route, Navigate } from 'react-router-dom';
import {
  SettingsLayout,
  AudioSettingsPage,
  LibrarySettingsPage,
  ReportBugSettingsPage,
  DataManagementSettingsPage,
} from '@soul-player/shared';
import { AboutSettingsPage } from './settings/AboutSettingsPage';
import { AppearanceSettingsPage } from './settings/AppearanceSettingsPage';
import { PlaybackSettingsPage } from './settings/PlaybackSettingsPage';
import { ShortcutsSettings } from '../components/ShortcutsSettings';

export function SettingsRouter() {
  return (
    <SettingsLayout>
      <Routes>
        {/* Redirect /settings to /settings/audio by default */}
        <Route index element={<Navigate to="/settings/audio" replace />} />

        {/* Settings pages */}
        <Route path="audio" element={<AudioSettingsPage />} />
        <Route path="library" element={<LibrarySettingsPage />} />
        <Route path="playback" element={<PlaybackSettingsPage />} />
        <Route path="appearance" element={<AppearanceSettingsPage />} />
        <Route path="shortcuts" element={<ShortcutsSettings />} />
        <Route path="data-management" element={<DataManagementSettingsPage />} />
        <Route path="report-bug" element={<ReportBugSettingsPage />} />
        <Route path="about" element={<AboutSettingsPage />} />

        {/* Fallback */}
        <Route path="*" element={<Navigate to="/settings/audio" replace />} />
      </Routes>
    </SettingsLayout>
  );
}
