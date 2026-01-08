import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export type SyncStatus = 'idle' | 'scanning' | 'extracting' | 'validating' | 'cleaning' | 'error';
export type SyncPhase = 'scanning' | 'metadata_extraction' | 'validation' | 'cleanup';

export interface SyncProgress {
  status: SyncStatus;
  phase: SyncPhase | null;
  totalItems: number;
  processedItems: number;
  successfulItems: number;
  failedItems: number;
  currentItem: string | null;
  percentage: number;
}

export interface SyncSummary {
  sessionId: string;
  startedAt: string;
  completedAt: string;
  durationSeconds: number;
  filesScanned: number;
  tracksUpdated: number;
  errorsEncountered: number;
  orphansCleaned: number;
}

interface SyncState {
  progress: SyncProgress | null;
  summary: SyncSummary | null;
  error: string | null;
  isSyncing: boolean;
  syncRequired: boolean;

  // Actions
  startSync: (trigger: 'manual' | 'migration' | 'source_activation') => Promise<void>;
  cancelSync: () => Promise<void>;
  fetchStatus: () => Promise<void>;
  setSyncRequired: (required: boolean) => void;

  // Internal
  setProgress: (progress: SyncProgress) => void;
  setSummary: (summary: SyncSummary) => void;
  setError: (error: string | null) => void;
}

export const useSyncStore = create<SyncState>((set, get) => ({
  progress: null,
  summary: null,
  error: null,
  isSyncing: false,
  syncRequired: false,

  startSync: async (trigger) => {
    try {
      await invoke('start_sync', { trigger });
      set({ isSyncing: true, error: null, syncRequired: false });
    } catch (err) {
      set({ error: String(err) });
    }
  },

  cancelSync: async () => {
    try {
      await invoke('cancel_sync');
    } catch (err) {
      set({ error: String(err) });
    }
  },

  fetchStatus: async () => {
    try {
      const progress = await invoke<SyncProgress>('get_sync_status');
      set({ progress, isSyncing: progress.status !== 'idle' });
    } catch (err) {
      set({ error: String(err) });
    }
  },

  setSyncRequired: (required) => set({ syncRequired: required }),
  setProgress: (progress) => set({ progress, isSyncing: progress.status !== 'idle' }),
  setSummary: (summary) => set({ summary, isSyncing: false }),
  setError: (error) => set({ error, isSyncing: false }),
}));

// Setup event listeners
export function setupSyncListeners() {
  listen<SyncProgress>('sync-progress', (event) => {
    useSyncStore.getState().setProgress(event.payload);
  });

  listen<SyncSummary>('sync-complete', (event) => {
    useSyncStore.getState().setSummary(event.payload);
  });

  listen<string>('sync-error', (event) => {
    useSyncStore.getState().setError(event.payload);
  });

  listen('sync-required', () => {
    useSyncStore.getState().setSyncRequired(true);
  });
}
