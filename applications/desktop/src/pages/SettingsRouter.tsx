/**
 * Settings Router - handles nested settings routes with sidebar layout
 */

import { Routes, Route, Navigate } from 'react-router-dom';
import {
  SettingsLayout,
  AudioSettingsPage,
} from '@soul-player/shared';
import { AboutSettingsPage } from './settings/AboutSettingsPage';
import { AppearanceSettingsPage } from './settings/AppearanceSettingsPage';
import { MusicDataSettingsPage } from './settings/MusicDataSettingsPage';
import { ShortcutsSettings } from '../components/ShortcutsSettings';

export function SettingsRouter() {
  return (
    <SettingsLayout>
      <Routes>
        <Route index element={<Navigate to="/settings/appearance" replace />} />

        <Route path="appearance" element={<AppearanceSettingsPage />} />
        <Route path="music-data" element={<MusicDataSettingsPage />} />
        <Route path="audio" element={<AudioSettingsPage />} />
        <Route path="shortcuts" element={<ShortcutsSettings />} />
        <Route path="report-bug" element={<Navigate to="/settings/about" replace />} />
        <Route path="about" element={<AboutSettingsPage />} />

        {/* Redirect old paths */}
        <Route path="library" element={<Navigate to="/settings/music-data" replace />} />
        <Route path="playback" element={<Navigate to="/settings/audio" replace />} />
        <Route path="data-management" element={<Navigate to="/settings/music-data" replace />} />

        <Route path="*" element={<Navigate to="/settings/appearance" replace />} />
      </Routes>
    </SettingsLayout>
  );
}
