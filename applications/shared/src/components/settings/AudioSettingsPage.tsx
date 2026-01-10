// Audio Settings Page with Pipeline Visualization

import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { AlertCircle, CheckCircle2 } from 'lucide-react';
import { PipelineVisualization } from './audio/PipelineVisualization';
import { BackendSelector } from './audio/BackendSelector';
import { DeviceSelector } from './audio/DeviceSelector';
import { DspConfig } from './audio/DspConfig';
import { UpsamplingSettings } from './audio/UpsamplingSettings';
import { VolumeLevelingSettings } from './audio/VolumeLevelingSettings';
import { BufferSettings } from './audio/BufferSettings';

export interface AudioBackend {
  backend: 'default' | 'asio' | 'jack';
  name: string;
  description: string;
  available: boolean;
  is_default: boolean;
  device_count: number;
}

export interface AudioDevice {
  name: string;
  backend: string;
  isDefault: boolean;
  sampleRate: number;
  channels: number;
  sampleRateRange?: [number, number];
}

export interface AudioSettings {
  backend: 'default' | 'asio' | 'jack';
  device_name: string | null;
  dsp_enabled: boolean;
  dsp_slots: (string | null)[];
  upsampling_quality: 'disabled' | 'fast' | 'balanced' | 'high' | 'maximum';
  upsampling_target_rate: 'auto' | number;
  volume_leveling_mode: 'disabled' | 'replaygain_track' | 'replaygain_album' | 'ebu_r128';
  preload_enabled: boolean;
  buffer_size: 'auto' | number;
}

export function AudioSettingsPage() {
  const { t } = useTranslation();

  const [backends, setBackends] = useState<AudioBackend[]>([]);
  const [devices, setDevices] = useState<AudioDevice[]>([]);
  const [settings, setSettings] = useState<AudioSettings>({
    backend: 'default',
    device_name: null,
    dsp_enabled: false,
    dsp_slots: [null, null, null, null],
    upsampling_quality: 'high',
    upsampling_target_rate: 'auto',
    volume_leveling_mode: 'disabled',
    preload_enabled: true,
    buffer_size: 'auto',
  });
  const [loading, setLoading] = useState(true);
  const [notification, setNotification] = useState<{ type: 'success' | 'error'; message: string } | null>(null);

  useEffect(() => {
    loadAudioSettings();
  }, []);

  // Auto-hide notification after 3 seconds
  useEffect(() => {
    if (notification) {
      const timer = setTimeout(() => setNotification(null), 3000);
      return () => clearTimeout(timer);
    }
  }, [notification]);

  const showNotification = (type: 'success' | 'error', message: string) => {
    setNotification({ type, message });
  };

  const loadAudioSettings = async () => {
    try {
      setLoading(true);

      // Load backends
      const backendsData = await invoke<AudioBackend[]>('get_audio_backends');
      setBackends(backendsData);

      // Load devices for current backend
      const currentBackend = settings.backend;
      const devicesData = await invoke<AudioDevice[]>('get_audio_devices', { backendStr: currentBackend });
      setDevices(devicesData);

      // Load settings from database
      const savedSettings = await invoke<string | null>('get_user_setting', {
        key: 'audio.pipeline'
      });

      if (savedSettings) {
        setSettings(JSON.parse(savedSettings));
      }
    } catch (error) {
      console.error('Failed to load audio settings:', error);
    } finally {
      setLoading(false);
    }
  };

  const updateSettings = async (updates: Partial<AudioSettings>) => {
    const newSettings = { ...settings, ...updates };
    setSettings(newSettings);

    try {
      await invoke('set_user_setting', {
        key: 'audio.pipeline',
        value: JSON.stringify(newSettings)
      });
    } catch (error) {
      console.error('Failed to save audio settings:', error);
    }
  };

  const handleBackendChange = async (backend: 'default' | 'asio' | 'jack') => {
    updateSettings({ backend });

    // Reload devices for new backend
    try {
      const devicesData = await invoke<AudioDevice[]>('get_audio_devices', { backendStr: backend });
      setDevices(devicesData);
    } catch (error) {
      console.error('Failed to load devices:', error);
    }
  };

  const handleDeviceChange = async (deviceName: string) => {
    updateSettings({ device_name: deviceName });

    try {
      await invoke('set_audio_device', {
        backendStr: settings.backend,
        deviceName
      });
      showNotification('success', `Switched to audio device: ${deviceName}`);
    } catch (error) {
      console.error('Failed to set audio device:', error);
      showNotification('error', `Failed to switch audio device: ${error}`);
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-muted-foreground">Loading audio settings...</div>
      </div>
    );
  }

  return (
    <div className="space-y-8">
      {/* Notification Toast */}
      {notification && (
        <div
          className={`
            fixed top-4 right-4 z-50 p-4 rounded-lg shadow-lg border flex items-center gap-3
            animate-in slide-in-from-top-2 duration-300
            ${notification.type === 'success'
              ? 'bg-green-50 border-green-200 text-green-900 dark:bg-green-950 dark:border-green-800 dark:text-green-100'
              : 'bg-red-50 border-red-200 text-red-900 dark:bg-red-950 dark:border-red-800 dark:text-red-100'
            }
          `}
        >
          {notification.type === 'success' ? (
            <CheckCircle2 className="w-5 h-5 flex-shrink-0" />
          ) : (
            <AlertCircle className="w-5 h-5 flex-shrink-0" />
          )}
          <span className="text-sm font-medium">{notification.message}</span>
        </div>
      )}

      {/* Page Header */}
      <div>
        <h1 className="text-3xl font-bold mb-2">{t('settings.audio')}</h1>
        <p className="text-muted-foreground">
          Configure high-quality audio processing pipeline
        </p>
      </div>

      {/* Pipeline Visualization */}
      <PipelineVisualization
        dspEnabled={settings.dsp_enabled}
        upsamplingEnabled={true}
        volumeLevelingEnabled={settings.volume_leveling_mode !== 'disabled'}
      />

      {/* Audio Driver Section */}
      <section className="space-y-4">
        <div>
          <h2 className="text-xl font-semibold mb-1">Audio Driver</h2>
          <p className="text-sm text-muted-foreground">
            Select audio backend and output device
          </p>
        </div>

        <div className="bg-card border border-border rounded-lg p-6 space-y-6">
          <BackendSelector
            backends={backends}
            currentBackend={settings.backend}
            onBackendChange={handleBackendChange}
          />

          <DeviceSelector
            devices={devices}
            currentDevice={settings.device_name}
            onDeviceChange={handleDeviceChange}
          />
        </div>
      </section>

      {/* DSP Effects Section */}
      <section className="space-y-4">
        <div>
          <h2 className="text-xl font-semibold mb-1">DSP Effects</h2>
          <p className="text-sm text-muted-foreground">
            Digital signal processing applied before volume control
          </p>
        </div>

        <DspConfig />
      </section>

      {/* Upsampling Section */}
      <section className="space-y-4">
        <div>
          <h2 className="text-xl font-semibold mb-1">Upsampling / Resampling</h2>
          <p className="text-sm text-muted-foreground">
            Automatic sample rate matching to prevent playback speed issues
          </p>
        </div>

        <UpsamplingSettings
          quality={settings.upsampling_quality}
          targetRate={settings.upsampling_target_rate}
          onQualityChange={(quality) => updateSettings({ upsampling_quality: quality })}
          onTargetRateChange={(rate) => updateSettings({ upsampling_target_rate: rate })}
        />
      </section>

      {/* Volume Leveling Section */}
      <section className="space-y-4">
        <div>
          <h2 className="text-xl font-semibold mb-1">Volume Leveling</h2>
          <p className="text-sm text-muted-foreground">
            Automatic loudness normalization (ReplayGain / EBU R128)
          </p>
        </div>

        <VolumeLevelingSettings
          mode={settings.volume_leveling_mode}
          onModeChange={(mode) => updateSettings({ volume_leveling_mode: mode })}
        />
      </section>

      {/* Buffer Settings Section */}
      <section className="space-y-4">
        <div>
          <h2 className="text-xl font-semibold mb-1">Buffer Settings</h2>
          <p className="text-sm text-muted-foreground">
            Configure audio buffering and pre-loading behavior
          </p>
        </div>

        <BufferSettings
          bufferSize={settings.buffer_size}
          preloadEnabled={settings.preload_enabled}
          onBufferSizeChange={(size) => updateSettings({ buffer_size: size })}
          onPreloadChange={(enabled) => updateSettings({ preload_enabled: enabled })}
        />
      </section>

      {/* Reset to Defaults */}
      <div className="pt-6 border-t">
        <button
          className="px-4 py-2 border border-border rounded-lg hover:bg-muted transition-colors"
          onClick={() => {
            if (confirm('Reset all audio settings to defaults?')) {
              updateSettings({
                backend: 'default',
                device_name: null,
                dsp_enabled: false,
                dsp_slots: [null, null, null, null],
                upsampling_quality: 'high',
                upsampling_target_rate: 'auto',
                volume_leveling_mode: 'disabled',
                preload_enabled: true,
                buffer_size: 'auto',
              });
            }
          }}
        >
          Reset to Defaults
        </button>
      </div>
    </div>
  );
}
