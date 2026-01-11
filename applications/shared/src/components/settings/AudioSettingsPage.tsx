// Audio Settings Page with Pipeline-based Layout
// Each stage shows description, current config, settings, and arrow to next stage

import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import {
  AlertCircle,
  CheckCircle2,
  RotateCcw,
  Volume2,
} from 'lucide-react';
import { usePlatform } from '../../contexts/PlatformContext';
import { ConfirmDialog } from '../ui/Dialog';
import { PipelineVisualization } from './audio/PipelineVisualization';
import { PipelineStage } from './audio/PipelineStage';
import { BackendSelector } from './audio/BackendSelector';
import { DeviceSelector } from './audio/DeviceSelector';
import { DspConfig } from './audio/DspConfig';
import { UpsamplingSettings } from './audio/UpsamplingSettings';
import { VolumeLevelingSettings } from './audio/VolumeLevelingSettings';
import { HeadroomSettings } from './audio/HeadroomSettings';
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
  resampling_quality: 'fast' | 'balanced' | 'high' | 'maximum';
  resampling_target_rate: 'auto' | number;
  resampling_backend: 'auto' | 'rubato' | 'r8brain';
  volume_leveling_mode: 'disabled' | 'replaygain_track' | 'replaygain_album' | 'ebu_r128';
  volume_leveling_preamp_db: number;
  volume_leveling_prevent_clipping: boolean;
  preload_enabled: boolean;
  buffer_size: 'auto' | number;
  crossfade_enabled: boolean;
  crossfade_duration_ms: number;
  crossfade_curve: 'linear' | 'logarithmic' | 's_curve' | 'equal_power';
}

export function AudioSettingsPage() {
  const { features } = usePlatform();

  // If audio settings are not available (web demo), show a simplified view
  if (!features.hasAudioSettings) {
    return <AudioSettingsDemoView />;
  }

  return <AudioSettingsDesktop />;
}

// Demo view for web - shows audio features without Tauri integration
function AudioSettingsDemoView() {
  const { t } = useTranslation();

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold mb-2">{t('settings.audio.title')}</h1>
        <p className="text-muted-foreground">
          Audio processing pipeline configuration
        </p>
      </div>

      {/* Demo Pipeline Overview */}
      <div className="bg-muted/30 rounded-lg p-6">
        <div className="flex items-center gap-4 mb-4">
          <div className="w-12 h-12 bg-primary/10 rounded-lg flex items-center justify-center">
            <Volume2 className="w-6 h-6 text-primary" />
          </div>
          <div>
            <h3 className="font-semibold">Professional Audio Pipeline</h3>
            <p className="text-sm text-muted-foreground">
              {t('settings.demoDisabled')}
            </p>
          </div>
        </div>

        <div className="space-y-4">
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <FeatureCard
              title="High-Quality Resampling"
              description="r8brain and Rubato algorithms for sample rate conversion"
            />
            <FeatureCard
              title="DSP Effects Chain"
              description="4-slot parametric EQ, compressor, limiter, crossfeed"
            />
            <FeatureCard
              title="Volume Leveling"
              description="ReplayGain (track/album) and EBU R128 normalization"
            />
            <FeatureCard
              title="Gapless Playback"
              description="Seamless transitions with crossfade support"
            />
            <FeatureCard
              title="ASIO & JACK Support"
              description="Low-latency audio output on Windows and Linux"
            />
            <FeatureCard
              title="Headroom Management"
              description="Automatic clipping prevention during DSP processing"
            />
          </div>
        </div>
      </div>
    </div>
  );
}

function FeatureCard({ title, description }: { title: string; description: string }) {
  return (
    <div className="bg-background rounded-lg p-4 border border-border">
      <h4 className="font-medium text-sm">{title}</h4>
      <p className="text-xs text-muted-foreground mt-1">{description}</p>
    </div>
  );
}

// Full audio settings for desktop with Tauri integration
function AudioSettingsDesktop() {
  const { t } = useTranslation();

  // Dynamic import of invoke to avoid errors on web
  const [invoke, setInvoke] = useState<typeof import('@tauri-apps/api/core').invoke | null>(null);

  useEffect(() => {
    import('@tauri-apps/api/core').then(mod => {
      setInvoke(() => mod.invoke);
    });
  }, []);

  const [backends, setBackends] = useState<AudioBackend[]>([]);
  const [devices, setDevices] = useState<AudioDevice[]>([]);
  const [settings, setSettings] = useState<AudioSettings>({
    backend: 'default',
    device_name: null,
    dsp_enabled: false,
    dsp_slots: [null, null, null, null],
    resampling_quality: 'high',
    resampling_target_rate: 'auto',
    resampling_backend: 'auto',
    volume_leveling_mode: 'disabled',
    volume_leveling_preamp_db: 0,
    volume_leveling_prevent_clipping: true,
    preload_enabled: true,
    buffer_size: 'auto',
    crossfade_enabled: false,
    crossfade_duration_ms: 3000,
    crossfade_curve: 'equal_power',
  });
  const [r8brainAvailable, setR8brainAvailable] = useState(false);
  const [loading, setLoading] = useState(true);
  const [notification, setNotification] = useState<{ type: 'success' | 'error'; message: string } | null>(null);
  const [dspEffectCount, setDspEffectCount] = useState(0);
  const [showResetDialog, setShowResetDialog] = useState(false);

  useEffect(() => {
    if (invoke) {
      loadAudioSettings();
    }
  }, [invoke]);

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
    if (!invoke) return;

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
        try {
          const parsed = JSON.parse(savedSettings);
          // Migrate old property names to new ones
          const migrated: AudioSettings = {
            backend: parsed.backend ?? 'default',
            device_name: parsed.device_name ?? null,
            dsp_enabled: parsed.dsp_enabled ?? false,
            dsp_slots: parsed.dsp_slots ?? [null, null, null, null],
            // Handle migration from old upsampling_* to new resampling_*
            resampling_quality: parsed.resampling_quality ?? parsed.upsampling_quality ?? 'high',
            resampling_target_rate: parsed.resampling_target_rate ?? parsed.upsampling_target_rate ?? 'auto',
            resampling_backend: parsed.resampling_backend ?? 'auto',
            volume_leveling_mode: parsed.volume_leveling_mode ?? 'disabled',
            volume_leveling_preamp_db: parsed.volume_leveling_preamp_db ?? 0,
            volume_leveling_prevent_clipping: parsed.volume_leveling_prevent_clipping ?? true,
            preload_enabled: parsed.preload_enabled ?? true,
            buffer_size: parsed.buffer_size ?? 'auto',
            crossfade_enabled: parsed.crossfade_enabled ?? false,
            crossfade_duration_ms: parsed.crossfade_duration_ms ?? 3000,
            crossfade_curve: parsed.crossfade_curve ?? 'equal_power',
          };
          // Filter out 'disabled' which is no longer valid for resampling_quality (migration from old settings)
          if ((migrated.resampling_quality as string) === 'disabled') {
            migrated.resampling_quality = 'high';
          }
          setSettings(migrated);
        } catch (e) {
          console.error('Failed to parse audio settings:', e);
        }
      }

      // Check if r8brain backend is available
      try {
        const r8brainStatus = await invoke<boolean>('is_r8brain_available');
        setR8brainAvailable(r8brainStatus);
      } catch {
        setR8brainAvailable(false);
      }
    } catch (error) {
      console.error('Failed to load audio settings:', error);
    } finally {
      setLoading(false);
    }
  };

  const updateSettings = async (updates: Partial<AudioSettings>) => {
    if (!invoke) return;

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
    if (!invoke) return;
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
    if (!invoke) return;
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

  const handleDspChainChange = () => {
    // Reload DSP chain count
    loadDspChainCount();
  };

  const loadDspChainCount = async () => {
    if (!invoke) return;
    try {
      const chain = await invoke<{ effect: unknown }[]>('get_dsp_chain');
      setDspEffectCount(chain.filter(slot => slot.effect !== null).length);
      setSettings(prev => ({ ...prev, dsp_enabled: chain.some(slot => slot.effect !== null) }));
    } catch {
      // Silently ignore if DSP not available
    }
  };

  useEffect(() => {
    if (invoke) {
      loadDspChainCount();
    }
  }, [invoke]);

  const resetToDefaults = async () => {
    if (!invoke) return;
    updateSettings({
      backend: 'default',
      device_name: null,
      dsp_enabled: false,
      dsp_slots: [null, null, null, null],
      resampling_quality: 'high',
      resampling_target_rate: 'auto',
      resampling_backend: 'auto',
      volume_leveling_mode: 'disabled',
      volume_leveling_preamp_db: 0,
      volume_leveling_prevent_clipping: true,
      preload_enabled: true,
      buffer_size: 'auto',
      crossfade_enabled: false,
      crossfade_duration_ms: 3000,
      crossfade_curve: 'equal_power',
    });
    // Also reset the backend settings via Tauri commands
    try {
      await invoke('set_volume_leveling_mode', { mode: 'disabled' });
      await invoke('set_volume_leveling_preamp', { preampDb: 0 });
      await invoke('set_volume_leveling_prevent_clipping', { prevent: true });
      // Reset crossfade settings to defaults
      await invoke('set_crossfade_settings', {
        enabled: false,
        durationMs: 3000,
        curve: 'equal_power',
      });
    } catch (error) {
      console.error('Failed to reset audio settings:', error);
    }
    setShowResetDialog(false);
  };

  // Get backend display name
  const getBackendName = () => {
    const backend = backends.find(b => b.backend === settings.backend);
    return backend?.name || settings.backend;
  };

  // Get volume leveling mode display
  const getVolumeLevelingDisplay = () => {
    switch (settings.volume_leveling_mode) {
      case 'replaygain_track': return 'RG Track';
      case 'replaygain_album': return 'RG Album';
      case 'ebu_r128': return 'EBU R128';
      default: return 'Off';
    }
  };

  // Handle preamp change
  const handlePreampChange = async (preampDb: number) => {
    if (!invoke) return;
    updateSettings({ volume_leveling_preamp_db: preampDb });
    try {
      await invoke('set_volume_leveling_preamp', { preampDb });
    } catch (error) {
      console.error('Failed to set preamp:', error);
    }
  };

  // Handle prevent clipping change
  const handlePreventClippingChange = async (prevent: boolean) => {
    if (!invoke) return;
    updateSettings({ volume_leveling_prevent_clipping: prevent });
    try {
      await invoke('set_volume_leveling_prevent_clipping', { prevent });
    } catch (error) {
      console.error('Failed to set prevent clipping:', error);
    }
  };

  // Handle crossfade changes with runtime application
  const handleCrossfadeChange = async (crossfade: {
    enabled: boolean;
    durationMs: number;
    curve: 'linear' | 'logarithmic' | 's_curve' | 'equal_power';
  }) => {
    if (!invoke) return;
    // Update local state and persist to JSON settings
    updateSettings({
      crossfade_enabled: crossfade.enabled,
      crossfade_duration_ms: crossfade.durationMs,
      crossfade_curve: crossfade.curve,
    });

    // Apply settings to audio engine immediately (no restart required)
    try {
      // Map frontend curve names to backend curve names
      const curveMapping: Record<string, string> = {
        'linear': 'linear',
        'logarithmic': 'square_root', // Backend uses square_root for this
        's_curve': 's_curve',
        'equal_power': 'equal_power',
      };

      await invoke('set_crossfade_settings', {
        enabled: crossfade.enabled,
        durationMs: crossfade.durationMs,
        curve: curveMapping[crossfade.curve] || 'equal_power',
      });
    } catch (error) {
      console.error('Failed to apply crossfade settings:', error);
      showNotification('error', 'Failed to apply crossfade settings');
    }
  };

  // Handle resampling quality change
  const handleResamplingQualityChange = async (quality: 'fast' | 'balanced' | 'high' | 'maximum') => {
    if (!invoke) return;
    // Update local state and persist to JSON
    updateSettings({ resampling_quality: quality });

    // Apply to audio engine (takes effect on next track)
    try {
      await invoke('set_resampling_quality', { quality });
      showNotification('success', t('settings.audio.resampling.applyOnNextTrack', 'Resampling settings will apply on next track'));
    } catch (error) {
      console.error('Failed to apply resampling quality:', error);
      showNotification('error', 'Failed to apply resampling quality');
    }
  };

  // Handle resampling target rate change
  const handleResamplingTargetRateChange = async (rate: 'auto' | number) => {
    if (!invoke) return;
    // Update local state and persist to JSON
    updateSettings({ resampling_target_rate: rate });

    // Apply to audio engine (takes effect on next track)
    try {
      // Convert 'auto' to 0 for backend
      const targetRate = rate === 'auto' ? 0 : rate;
      await invoke('set_resampling_target_rate', { rate: targetRate });
      showNotification('success', t('settings.audio.resampling.applyOnNextTrack', 'Resampling settings will apply on next track'));
    } catch (error) {
      console.error('Failed to apply resampling target rate:', error);
      showNotification('error', 'Failed to apply resampling target rate');
    }
  };

  // Handle resampling backend change
  const handleResamplingBackendChange = async (backend: 'auto' | 'rubato' | 'r8brain') => {
    if (!invoke) return;
    // Update local state and persist to JSON
    updateSettings({ resampling_backend: backend });

    // Apply to audio engine (takes effect on next track)
    try {
      await invoke('set_resampling_backend', { backend });
      showNotification('success', t('settings.audio.resampling.applyOnNextTrack', 'Resampling settings will apply on next track'));
    } catch (error) {
      console.error('Failed to apply resampling backend:', error);
      showNotification('error', 'Failed to apply resampling backend');
    }
  };

  // Show loading state while Tauri invoke is being loaded
  if (!invoke) {
    return (
      <div className="flex items-center justify-center py-12">
        <div className="text-muted-foreground">Loading audio settings...</div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
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
      <div className="flex items-start justify-between">
        <div>
          <h1 className="text-3xl font-bold mb-2">{t('settings.audio.title')}</h1>
          <p className="text-muted-foreground">
            Configure your audio processing pipeline stage by stage
          </p>
        </div>

        {/* Reset Button */}
        <button
          onClick={() => setShowResetDialog(true)}
          className="flex items-center gap-2 px-3 py-2 text-sm border border-border rounded-lg hover:bg-muted transition-colors"
        >
          <RotateCcw className="w-4 h-4" />
          Reset All
        </button>
      </div>

      {/* Reset Confirmation Dialog */}
      <ConfirmDialog
        open={showResetDialog}
        onClose={() => setShowResetDialog(false)}
        onConfirm={resetToDefaults}
        title="Reset Audio Settings"
        message="This will reset all audio settings to their default values. Your current configuration will be lost."
        confirmText="Reset"
        variant="destructive"
      />

      {/* Pipeline Overview */}
      <PipelineVisualization
        backend={getBackendName()}
        deviceName={settings.device_name}
        dspEnabled={settings.dsp_enabled}
        dspEffectCount={dspEffectCount}
        upsamplingEnabled={true}
        upsamplingRate={settings.resampling_quality.charAt(0).toUpperCase() + settings.resampling_quality.slice(1)}
        volumeLevelingEnabled={settings.volume_leveling_mode !== 'disabled'}
        volumeLevelingMode={getVolumeLevelingDisplay()}
        loading={loading}
      />

      {/* Pipeline Stages - Order matches overview: Resample → DSP → Leveling → Buffer → Output */}
      <div>
        {/* Stage 1: Resampling */}
        <PipelineStage
          id="audio-stage-1"
          title="Resampling"
          description="Automatic sample rate conversion to match your output device"
          isActive={true}
          currentConfig={settings.resampling_quality.charAt(0).toUpperCase() + settings.resampling_quality.slice(1)}
          statusText={settings.resampling_backend === 'auto' ? 'Auto' : settings.resampling_backend}
        >
          <UpsamplingSettings
            quality={settings.resampling_quality}
            targetRate={settings.resampling_target_rate}
            backend={settings.resampling_backend}
            r8brainAvailable={r8brainAvailable}
            onQualityChange={handleResamplingQualityChange}
            onTargetRateChange={handleResamplingTargetRateChange}
            onBackendChange={handleResamplingBackendChange}
          />
        </PipelineStage>

        {/* Stage 2: DSP Effects */}
        <PipelineStage
          id="audio-stage-2"
          title="DSP Effects"
          description="Digital signal processing - EQ, compression, and effects applied to audio"
          isActive={settings.dsp_enabled}
          isOptional={true}
          currentConfig={dspEffectCount > 0 ? `${dspEffectCount} active` : 'None'}
          statusText={settings.dsp_enabled ? 'Enabled' : 'Disabled'}
        >
          <DspConfig onChainChange={handleDspChainChange} />
        </PipelineStage>

        {/* Stage 3: Volume Leveling */}
        <PipelineStage
          id="audio-stage-3"
          title="Volume Leveling"
          description="Automatic loudness normalization using ReplayGain or EBU R128"
          isActive={settings.volume_leveling_mode !== 'disabled'}
          isOptional={true}
          currentConfig={getVolumeLevelingDisplay()}
          statusText={settings.volume_leveling_mode !== 'disabled' ? 'Enabled' : 'Disabled'}
        >
          <VolumeLevelingSettings
            mode={settings.volume_leveling_mode}
            preampDb={settings.volume_leveling_preamp_db}
            preventClipping={settings.volume_leveling_prevent_clipping}
            onModeChange={async (mode) => {
              if (!invoke) return;
              // First apply to audio engine immediately
              try {
                await invoke('set_volume_leveling_mode', { mode });
              } catch (error) {
                console.error('Failed to set volume leveling mode:', error);
              }
              // Then persist to settings
              updateSettings({ volume_leveling_mode: mode });
            }}
            onPreampChange={handlePreampChange}
            onPreventClippingChange={handlePreventClippingChange}
          />
        </PipelineStage>

        {/* Stage 4: Headroom Management */}
        <PipelineStage
          id="audio-stage-4"
          title="Headroom Management"
          description="Prevents clipping by attenuating signal before DSP processing"
          isActive={true}
          isOptional={true}
          currentConfig="Auto"
          statusText="Active"
        >
          <HeadroomSettings />
        </PipelineStage>

        {/* Stage 5: Buffer Settings */}
        <PipelineStage
          id="audio-stage-5"
          title="Buffer & Performance"
          description="Configure audio buffering and pre-loading for optimal playback"
          isActive={true}
          currentConfig={settings.buffer_size === 'auto' ? 'Auto' : `${settings.buffer_size} samples`}
          statusText={settings.preload_enabled ? 'Preload On' : 'Streaming'}
        >
          <BufferSettings
            bufferSize={settings.buffer_size}
            preloadEnabled={settings.preload_enabled}
            crossfade={{
              enabled: settings.crossfade_enabled,
              durationMs: settings.crossfade_duration_ms,
              curve: settings.crossfade_curve,
            }}
            onBufferSizeChange={(size) => updateSettings({ buffer_size: size })}
            onPreloadChange={(enabled) => updateSettings({ preload_enabled: enabled })}
            onCrossfadeChange={handleCrossfadeChange}
          />
        </PipelineStage>

        {/* Stage 6: Audio Output (Backend & Device) */}
        <PipelineStage
          id="audio-stage-6"
          title="Audio Output"
          description="Select your audio driver backend and output device for playback"
          isActive={true}
          isLast={true}
        >
          <div className="space-y-6">
            <BackendSelector
              backends={backends}
              currentBackend={settings.backend}
              onBackendChange={handleBackendChange}
              loading={loading}
            />
            <DeviceSelector
              devices={devices}
              currentDevice={settings.device_name}
              onDeviceChange={handleDeviceChange}
              loading={loading}
            />
          </div>
        </PipelineStage>
      </div>
    </div>
  );
}
