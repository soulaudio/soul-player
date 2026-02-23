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
import { useBackend } from '../../contexts/BackendContext';
import { useAudioDevice } from '../../hooks/useAudioDevice';
import { ConfirmDialog } from '../ui/Dialog';
import { PipelineVisualization } from './audio/PipelineVisualization';
import { PipelineStage } from './audio/PipelineStage';
import { BackendSelector } from './audio/BackendSelector';
import { DeviceSelector } from '../sidebar/DeviceSelector';
import { DspConfig } from './audio/DspConfig';
import { UpsamplingSettings } from './audio/UpsamplingSettings';
import { VolumeLevelingSettings } from './audio/VolumeLevelingSettings';
import { HeadroomSettings } from './audio/HeadroomSettings';
import { BufferSettings } from './audio/BufferSettings';
import { debug } from '../../utils/debug';

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
        <h1 className="text-2xl font-bold mb-6">{t('settings.audio.title')}</h1>
        <p className="text-muted-foreground">
          {t('settings.audio.pipelineDescription')}
        </p>
      </div>

      {/* Demo Pipeline Overview */}
      <div className="bg-muted/30 rounded-lg p-6">
        <div className="flex items-center gap-4 mb-4">
          <div className="w-12 h-12 bg-primary/10 rounded-lg flex items-center justify-center">
            <Volume2 className="w-6 h-6 text-primary" />
          </div>
          <div>
            <h3 className="font-semibold">{t('settings.audio.professionalPipeline')}</h3>
            <p className="text-sm text-muted-foreground">
              {t('settings.demoDisabled')}
            </p>
          </div>
        </div>

        <div className="space-y-4">
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <FeatureCard
              title={t('settings.audio.features.resampling.title')}
              description={t('settings.audio.features.resampling.description')}
            />
            <FeatureCard
              title={t('settings.audio.features.dspChain.title')}
              description={t('settings.audio.features.dspChain.description')}
            />
            <FeatureCard
              title={t('settings.audio.features.volumeLeveling.title')}
              description={t('settings.audio.features.volumeLeveling.description')}
            />
            <FeatureCard
              title={t('settings.audio.features.gaplessPlayback.title')}
              description={t('settings.audio.features.gaplessPlayback.description')}
            />
            <FeatureCard
              title={t('settings.audio.features.asioJack.title')}
              description={t('settings.audio.features.asioJack.description')}
            />
            <FeatureCard
              title={t('settings.audio.features.headroom.title')}
              description={t('settings.audio.features.headroom.description')}
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
  const backend = useBackend();

  const {
    backends,
    devices,
    currentDevice: activeDevice,
    isLoading: isLoadingDevices,
    switchDevice: switchAudioDevice,
    loadBackend,
  } = useAudioDevice(true)

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
    loadAudioSettings();
  }, []);

  // Sync the backend picker display with the actually playing device on first load
  useEffect(() => {
    if (activeDevice?.backend && !loading) {
      setSettings(prev => ({
        ...prev,
        backend: activeDevice.backend as 'default' | 'asio' | 'jack',
      }))
    }
  }, [activeDevice?.backend, loading])

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

      // Load settings from database
      const savedSettings = await backend.getUserSetting('audio.pipeline');

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
          debug.error('Failed to parse audio settings:', e);
        }
      }

      // Check if r8brain backend is available
      try {
        const r8brainStatus = await backend.isR8brainAvailable();
        setR8brainAvailable(r8brainStatus);
      } catch {
        setR8brainAvailable(false);
      }
    } catch (error) {
      debug.error('Failed to load audio settings:', error);
    } finally {
      setLoading(false);
    }
  };

  const updateSettings = async (updates: Partial<AudioSettings>) => {
    const newSettings = { ...settings, ...updates };
    setSettings(newSettings);

    try {
      await backend.setUserSetting('audio.pipeline', JSON.stringify(newSettings));
    } catch (error) {
      debug.error('Failed to save audio settings:', error);
    }
  };

  const handleBackendChange = async (selectedBackend: 'default' | 'asio' | 'jack') => {
    updateSettings({ backend: selectedBackend })
    await loadBackend(selectedBackend)
  };

  const handleSwitchDevice = async (backendStr: string, deviceName: string) => {
    updateSettings({ backend: backendStr as 'default' | 'asio' | 'jack', device_name: deviceName })
    try {
      await switchAudioDevice(backendStr, deviceName)
      showNotification('success', t('settings.audio.deviceSwitched', { name: deviceName }))
    } catch (error) {
      debug.error('Failed to set audio device:', error)
      showNotification('error', t('settings.audio.deviceSwitchFailed', { error: String(error) }))
    }
  };

  const handleDspChainChange = () => {
    // Reload DSP chain count
    loadDspChainCount();
  };

  const loadDspChainCount = async () => {
    try {
      const chain = await backend.getDspChain();
      setDspEffectCount(chain.filter(slot => slot.effect !== null).length);
      setSettings(prev => ({ ...prev, dsp_enabled: chain.some(slot => slot.effect !== null) }));
    } catch {
      // Silently ignore if DSP not available
    }
  };

  useEffect(() => {
    loadDspChainCount();
  }, []);

  const resetToDefaults = async () => {
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
    // Also reset the backend settings via backend methods
    try {
      await backend.setVolumeLevelingMode('disabled');
      await backend.setVolumeLevelingPreamp(0);
      await backend.setVolumeLevelingPreventClipping(true);
      // Reset crossfade settings to defaults
      await backend.setCrossfadeSettings(false, 3000, 'equal_power');
    } catch (error) {
      debug.error('Failed to reset audio settings:', error);
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
      case 'replaygain_track': return t('settings.audio.volumeLeveling.rgTrack');
      case 'replaygain_album': return t('settings.audio.volumeLeveling.rgAlbum');
      case 'ebu_r128': return 'EBU R128';
      default: return t('settings.audio.volumeLeveling.off');
    }
  };

  // Handle preamp change
  const handlePreampChange = async (preampDb: number) => {
    updateSettings({ volume_leveling_preamp_db: preampDb });
    try {
      await backend.setVolumeLevelingPreamp(preampDb);
    } catch (error) {
      debug.error('Failed to set preamp:', error);
    }
  };

  // Handle prevent clipping change
  const handlePreventClippingChange = async (prevent: boolean) => {
    updateSettings({ volume_leveling_prevent_clipping: prevent });
    try {
      await backend.setVolumeLevelingPreventClipping(prevent);
    } catch (error) {
      debug.error('Failed to set prevent clipping:', error);
    }
  };

  // Handle crossfade changes with runtime application
  const handleCrossfadeChange = async (crossfade: {
    enabled: boolean;
    durationMs: number;
    curve: 'linear' | 'logarithmic' | 's_curve' | 'equal_power';
  }) => {
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

      await backend.setCrossfadeSettings(
        crossfade.enabled,
        crossfade.durationMs,
        curveMapping[crossfade.curve] || 'equal_power'
      );
    } catch (error) {
      debug.error('Failed to apply crossfade settings:', error);
      showNotification('error', t('settings.audio.crossfadeApplyFailed'));
    }
  };

  // Handle resampling quality change
  const handleResamplingQualityChange = async (quality: 'fast' | 'balanced' | 'high' | 'maximum') => {
    // Update local state and persist to JSON
    updateSettings({ resampling_quality: quality });

    // Apply to audio engine (takes effect on next track)
    try {
      await backend.setResamplingQuality(quality);
      showNotification('success', t('settings.audio.resampling.applyOnNextTrack', 'Resampling settings will apply on next track'));
    } catch (error) {
      debug.error('Failed to apply resampling quality:', error);
      showNotification('error', t('settings.audio.resamplingQualityFailed'));
    }
  };

  // Handle resampling target rate change
  const handleResamplingTargetRateChange = async (rate: 'auto' | number) => {
    // Update local state and persist to JSON
    updateSettings({ resampling_target_rate: rate });

    // Apply to audio engine (takes effect on next track)
    try {
      // Convert 'auto' to 0 for backend
      const targetRate = rate === 'auto' ? 0 : rate;
      await backend.setResamplingTargetRate(targetRate);
      showNotification('success', t('settings.audio.resampling.applyOnNextTrack', 'Resampling settings will apply on next track'));
    } catch (error) {
      debug.error('Failed to apply resampling target rate:', error);
      showNotification('error', t('settings.audio.resamplingTargetRateFailed'));
    }
  };

  // Handle resampling backend change
  const handleResamplingBackendChange = async (resamplingBackend: 'auto' | 'rubato' | 'r8brain') => {
    // Update local state and persist to JSON
    updateSettings({ resampling_backend: resamplingBackend });

    // Apply to audio engine (takes effect on next track)
    try {
      await backend.setResamplingBackend(resamplingBackend);
      showNotification('success', t('settings.audio.resampling.applyOnNextTrack', 'Resampling settings will apply on next track'));
    } catch (error) {
      debug.error('Failed to apply resampling backend:', error);
      showNotification('error', t('settings.audio.resamplingBackendFailed'));
    }
  };

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

      {/* Warning Banner */}
      <div className="bg-amber-50 dark:bg-amber-950/20 border border-amber-200 dark:border-amber-800 rounded-lg p-4">
        <div className="flex items-start gap-3">
          <AlertCircle className="w-5 h-5 text-amber-600 dark:text-amber-500 flex-shrink-0 mt-0.5" />
          <div className="flex-1">
            <h3 className="text-sm font-semibold text-amber-900 dark:text-amber-200 mb-1">
              {t('settings.audio.warning.title')}
            </h3>
            <p className="text-sm text-amber-800 dark:text-amber-300">
              {t('settings.audio.warning.description')}
            </p>
          </div>
        </div>
      </div>

      {/* Page Header */}
      <div className="flex items-start justify-between">
        <div>
          <h1 className="text-2xl font-bold mb-6">{t('settings.audio.title')}</h1>
          <p className="text-muted-foreground">
            {t('settings.audio.pipelineConfigDescription')}
          </p>
        </div>

        {/* Reset Button */}
        <button
          onClick={() => setShowResetDialog(true)}
          className="flex items-center gap-2 px-3 py-2 text-sm border border-border rounded-lg hover:bg-foreground/[var(--hover-bg-opacity)] transition-colors duration-[var(--transition-duration)]"
        >
          <RotateCcw className="w-4 h-4" />
          {t('settings.audio.resetAll')}
        </button>
      </div>

      {/* Reset Confirmation Dialog */}
      <ConfirmDialog
        open={showResetDialog}
        onClose={() => setShowResetDialog(false)}
        onConfirm={resetToDefaults}
        title={t('settings.audio.resetDialog.title')}
        message={t('settings.audio.resetDialog.message')}
        confirmText={t('settings.audio.resetDialog.confirm')}
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
          title={t('settings.audio.stages.resampling.title')}
          description={t('settings.audio.stages.resampling.description')}
          isActive={true}
          currentConfig={settings.resampling_quality.charAt(0).toUpperCase() + settings.resampling_quality.slice(1)}
          statusText={settings.resampling_backend === 'auto' ? t('settings.audio.auto') : settings.resampling_backend}
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
          title={t('settings.audio.stages.dspEffects.title')}
          description={t('settings.audio.stages.dspEffects.description')}
          isActive={settings.dsp_enabled}
          isOptional={true}
          currentConfig={dspEffectCount > 0 ? t('settings.audio.stages.dspEffects.activeCount', { count: dspEffectCount }) : t('settings.audio.stages.dspEffects.none')}
          statusText={settings.dsp_enabled ? t('common.enabled') : t('common.disabled')}
        >
          <DspConfig onChainChange={handleDspChainChange} />
        </PipelineStage>

        {/* Stage 3: Volume Leveling */}
        <PipelineStage
          id="audio-stage-3"
          title={t('settings.audio.stages.volumeLeveling.title')}
          description={t('settings.audio.stages.volumeLeveling.description')}
          isActive={settings.volume_leveling_mode !== 'disabled'}
          isOptional={true}
          currentConfig={getVolumeLevelingDisplay()}
          statusText={settings.volume_leveling_mode !== 'disabled' ? t('common.enabled') : t('common.disabled')}
        >
          <VolumeLevelingSettings
            mode={settings.volume_leveling_mode}
            preampDb={settings.volume_leveling_preamp_db}
            preventClipping={settings.volume_leveling_prevent_clipping}
            onModeChange={async (mode) => {
              // First apply to audio engine immediately
              try {
                await backend.setVolumeLevelingMode(mode);
              } catch (error) {
                debug.error('Failed to set volume leveling mode:', error);
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
          title={t('settings.audio.stages.headroom.title')}
          description={t('settings.audio.stages.headroom.description')}
          isActive={true}
          isOptional={true}
          currentConfig={t('settings.audio.auto')}
          statusText={t('settings.audio.stages.headroom.active')}
        >
          <HeadroomSettings />
        </PipelineStage>

        {/* Stage 5: Buffer Settings */}
        <PipelineStage
          id="audio-stage-5"
          title={t('settings.audio.stages.buffer.title')}
          description={t('settings.audio.stages.buffer.description')}
          isActive={true}
          currentConfig={settings.buffer_size === 'auto' ? t('settings.audio.auto') : t('settings.audio.stages.buffer.samples', { count: settings.buffer_size })}
          statusText={settings.preload_enabled ? t('settings.audio.stages.buffer.preloadOn') : t('settings.audio.stages.buffer.streaming')}
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
          title={t('settings.audio.stages.output.title')}
          description={t('settings.audio.stages.output.description')}
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
              currentDevice={activeDevice}
              backends={backends}
              devices={devices}
              isLoadingDevices={isLoadingDevices}
              hasRealDevices={true}
              onLoadDevices={() => {}}
              onSwitchDevice={handleSwitchDevice}
              variant="list"
            />
          </div>
        </PipelineStage>
      </div>
    </div>
  );
}
