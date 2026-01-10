/**
 * Demo Settings Page - matches desktop UI structure exactly
 * Uses same layout and components but with disabled states for browser demo
 */
'use client'

import { useState } from 'react'
import { Kbd } from '@soul-player/shared'
import { ThemePicker } from '@soul-player/shared/theme'
import {
  Settings,
  Volume2,
  Keyboard,
  Info,
  ChevronDown,
  RotateCcw,
} from 'lucide-react'
import { GITHUB_REPO } from '../../constants/links'

type SettingsTab = 'general' | 'audio' | 'shortcuts' | 'about'

interface NavItem {
  id: SettingsTab
  label: string
  icon: React.ComponentType<{ className?: string }>
}

const navigationItems: NavItem[] = [
  { id: 'general', label: 'General', icon: Settings },
  { id: 'audio', label: 'Audio', icon: Volume2 },
  { id: 'shortcuts', label: 'Shortcuts', icon: Keyboard },
  { id: 'about', label: 'About', icon: Info },
]

export function SettingsPage() {
  const [activeTab, setActiveTab] = useState<SettingsTab>('general')
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false)

  const activeItem = navigationItems.find((item) => item.id === activeTab)

  return (
    <div className="h-full flex flex-col md:flex-row">
      {/* Desktop Sidebar - hidden on mobile */}
      <aside className="hidden md:flex w-56 flex-shrink-0 flex-col">
        <nav className="p-4 h-full flex flex-col">
          <ul className="space-y-1">
            {navigationItems.map((item) => {
              const Icon = item.icon
              const isActive = activeTab === item.id

              return (
                <li key={item.id}>
                  <button
                    onClick={() => setActiveTab(item.id)}
                    className={`
                      w-full flex items-center gap-3 px-3 py-2 rounded-lg text-sm
                      transition-colors duration-150 text-left
                      ${
                        isActive
                          ? 'bg-primary text-primary-foreground font-medium'
                          : 'text-muted-foreground hover:bg-muted hover:text-foreground'
                      }
                    `}
                  >
                    <Icon className="w-4 h-4 flex-shrink-0" />
                    <span>{item.label}</span>
                  </button>
                </li>
              )
            })}
          </ul>
        </nav>
      </aside>

      {/* Mobile Header - visible only on mobile */}
      <div className="md:hidden">
        <div className="relative">
          <button
            onClick={() => setMobileMenuOpen(!mobileMenuOpen)}
            className="w-full flex items-center justify-between px-4 py-3 text-left"
          >
            <div className="flex items-center gap-3">
              {activeItem && <activeItem.icon className="w-5 h-5" />}
              <span className="font-medium">{activeItem?.label}</span>
            </div>
            <ChevronDown
              className={`w-5 h-5 text-muted-foreground transition-transform ${
                mobileMenuOpen ? 'rotate-180' : ''
              }`}
            />
          </button>

          {/* Mobile Dropdown Menu */}
          {mobileMenuOpen && (
            <div className="absolute top-full left-0 right-0 bg-background shadow-lg z-50">
              {navigationItems.map((item) => {
                const Icon = item.icon
                const isActive = activeTab === item.id

                return (
                  <button
                    key={item.id}
                    onClick={() => {
                      setActiveTab(item.id)
                      setMobileMenuOpen(false)
                    }}
                    className={`
                      w-full flex items-center gap-3 px-4 py-3 text-left
                      transition-colors duration-150
                      ${
                        isActive
                          ? 'bg-primary/10 text-primary font-medium'
                          : 'text-foreground hover:bg-muted'
                      }
                    `}
                  >
                    <Icon className="w-4 h-4 flex-shrink-0" />
                    <span>{item.label}</span>
                  </button>
                )
              })}
            </div>
          )}
        </div>
      </div>

      {/* Main Content */}
      <main className="flex-1 overflow-y-auto">
        <div className="max-w-4xl mx-auto p-4 md:p-8">
          {activeTab === 'general' && <GeneralSettings />}
          {activeTab === 'audio' && <AudioSettings />}
          {activeTab === 'shortcuts' && <ShortcutsSettings />}
          {activeTab === 'about' && <AboutSettings />}
        </div>
      </main>
    </div>
  )
}

// General Settings Tab Content - matches desktop structure
function GeneralSettings() {
  return (
    <div className="space-y-8">
      {/* Appearance Section */}
      <section>
        <h2 className="text-xl font-semibold mb-4">Appearance</h2>
        <div className="space-y-6">
          <div>
            <label className="block text-sm font-medium mb-2">Theme</label>
            <ThemePicker
              showImportExport={false}
              showAccessibilityInfo={true}
            />
          </div>

          <div>
            <label className="block text-sm font-medium mb-2">Language</label>
            <select
              className="w-full max-w-xs px-3 py-2 rounded-lg bg-muted opacity-60 cursor-not-allowed"
              defaultValue="en-US"
              disabled
            >
              <option value="en-US">English (US)</option>
              <option value="de">Deutsch</option>
              <option value="ja">日本語</option>
            </select>
            <p className="text-xs text-muted-foreground mt-1">
              Language selection available in desktop app
            </p>
          </div>

          <div>
            <label className="flex items-start space-x-3 cursor-not-allowed opacity-60">
              <input
                type="checkbox"
                defaultChecked={true}
                className="w-4 h-4 mt-0.5"
                disabled
              />
              <div>
                <span className="text-sm font-medium block">Show keyboard shortcuts</span>
                <p className="text-xs text-muted-foreground mt-1">
                  Display keyboard shortcuts in tooltips and UI elements. For example: <Kbd keys={['mod', 'k']} size="sm" />
                </p>
              </div>
            </label>
          </div>
        </div>
      </section>

      {/* Updates Section */}
      <section>
        <h2 className="text-xl font-semibold mb-4">Updates</h2>
        <div className="space-y-4">
          <label className="flex items-center space-x-3 cursor-not-allowed opacity-60">
            <input
              type="checkbox"
              defaultChecked={true}
              className="w-4 h-4"
              disabled
            />
            <span className="text-sm">Automatically check for updates</span>
          </label>

          <label className="flex items-center space-x-3 cursor-not-allowed opacity-60">
            <input
              type="checkbox"
              defaultChecked={false}
              className="w-4 h-4"
              disabled
            />
            <span className="text-sm">Install updates silently in background</span>
          </label>

          <button
            className="px-4 py-2 bg-primary text-primary-foreground rounded-lg opacity-50 cursor-not-allowed"
            disabled
          >
            Check for Updates
          </button>
          <p className="text-xs text-muted-foreground">
            Update checking available in desktop app
          </p>
        </div>
      </section>
    </div>
  )
}

// Audio Settings - Pipeline-based layout matching desktop
function AudioSettings() {
  return (
    <div className="space-y-6">
      {/* Page Header */}
      <div className="flex items-start justify-between">
        <div>
          <h1 className="text-3xl font-bold mb-2">Audio</h1>
          <p className="text-muted-foreground">
            Configure your audio processing pipeline stage by stage
          </p>
        </div>

        {/* Reset Button */}
        <button
          className="flex items-center gap-2 px-3 py-2 text-sm border border-border rounded-lg opacity-50 cursor-not-allowed"
          disabled
        >
          <RotateCcw className="w-4 h-4" />
          Reset All
        </button>
      </div>

      {/* Pipeline Overview */}
      <div className="bg-card rounded-xl border p-4 sm:p-6">
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-2">
            <div className="w-2 h-2 rounded-full bg-green-500 animate-pulse" />
            <span className="text-sm font-medium">Audio Pipeline</span>
          </div>
          <span className="text-xs text-muted-foreground">Demo Mode</span>
        </div>

        <div className="flex items-center gap-1 sm:gap-2 text-xs overflow-x-auto pb-2">
          <span className="px-2 py-1 rounded bg-primary/10 text-primary whitespace-nowrap">Source</span>
          <span className="text-muted-foreground">→</span>
          <span className="px-2 py-1 rounded bg-muted whitespace-nowrap">Resample</span>
          <span className="text-muted-foreground">→</span>
          <span className="px-2 py-1 rounded bg-muted whitespace-nowrap">DSP</span>
          <span className="text-muted-foreground">→</span>
          <span className="px-2 py-1 rounded bg-muted whitespace-nowrap">Leveling</span>
          <span className="text-muted-foreground">→</span>
          <span className="px-2 py-1 rounded bg-muted whitespace-nowrap">Buffer</span>
          <span className="text-muted-foreground">→</span>
          <span className="px-2 py-1 rounded bg-primary/10 text-primary whitespace-nowrap">Output</span>
        </div>
      </div>

      {/* Stage 1: Resampling */}
      <PipelineStage
        title="Resampling"
        description="Automatic sample rate conversion to match your output device"
        statusText="Auto"
        currentConfig="High"
      >
        <div className="space-y-4">
          <div>
            <label className="block text-sm font-medium mb-2">Quality</label>
            <select className="w-full max-w-xs px-3 py-2 border rounded-lg bg-background opacity-60 cursor-not-allowed" disabled>
              <option>High (Recommended)</option>
              <option>Fast</option>
              <option>Balanced</option>
              <option>Maximum</option>
            </select>
          </div>
          <div>
            <label className="block text-sm font-medium mb-2">Target Sample Rate</label>
            <select className="w-full max-w-xs px-3 py-2 border rounded-lg bg-background opacity-60 cursor-not-allowed" disabled>
              <option>Auto (Match Device)</option>
              <option>96 kHz</option>
              <option>192 kHz</option>
            </select>
          </div>
          <div>
            <label className="block text-sm font-medium mb-2">Backend</label>
            <select className="w-full max-w-xs px-3 py-2 border rounded-lg bg-background opacity-60 cursor-not-allowed" disabled>
              <option>Auto (Best Available)</option>
              <option>Rubato</option>
              <option>r8brain</option>
            </select>
          </div>
        </div>
      </PipelineStage>

      {/* Stage 2: DSP Effects */}
      <PipelineStage
        title="DSP Effects"
        description="Digital signal processing - EQ, compression, and effects applied to audio"
        statusText="Disabled"
        currentConfig="None"
        isOptional
      >
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <span className="font-medium">Enable DSP Processing</span>
            <input type="checkbox" className="w-4 h-4 opacity-60 cursor-not-allowed" disabled />
          </div>
          <div className="grid grid-cols-2 gap-3">
            <div className="p-3 border border-dashed rounded-lg text-center text-sm text-muted-foreground">
              EQ Slot 1
            </div>
            <div className="p-3 border border-dashed rounded-lg text-center text-sm text-muted-foreground">
              EQ Slot 2
            </div>
            <div className="p-3 border border-dashed rounded-lg text-center text-sm text-muted-foreground">
              Compressor
            </div>
            <div className="p-3 border border-dashed rounded-lg text-center text-sm text-muted-foreground">
              Limiter
            </div>
          </div>
          <p className="text-xs text-muted-foreground">
            Full DSP configuration available in desktop app
          </p>
        </div>
      </PipelineStage>

      {/* Stage 3: Volume Leveling */}
      <PipelineStage
        title="Volume Leveling"
        description="Automatic loudness normalization using ReplayGain or EBU R128"
        statusText="Disabled"
        currentConfig="Off"
        isOptional
      >
        <div>
          <label className="block text-sm font-medium mb-2">Mode</label>
          <select className="w-full max-w-xs px-3 py-2 border rounded-lg bg-background opacity-60 cursor-not-allowed" disabled>
            <option>Disabled</option>
            <option>ReplayGain (Track)</option>
            <option>ReplayGain (Album)</option>
            <option>EBU R128</option>
          </select>
        </div>
      </PipelineStage>

      {/* Stage 4: Buffer & Performance */}
      <PipelineStage
        title="Buffer & Performance"
        description="Configure audio buffering and pre-loading for optimal playback"
        statusText="Preload On"
        currentConfig="Auto"
      >
        <div className="space-y-4">
          <div>
            <label className="block text-sm font-medium mb-2">Buffer Size</label>
            <select className="w-full max-w-xs px-3 py-2 border rounded-lg bg-background opacity-60 cursor-not-allowed" disabled>
              <option>Auto</option>
              <option>512 samples</option>
              <option>1024 samples</option>
              <option>2048 samples</option>
              <option>4096 samples</option>
            </select>
          </div>
          <div className="flex items-center justify-between">
            <div>
              <span className="font-medium block">Enable Preloading</span>
              <span className="text-xs text-muted-foreground">Buffer upcoming tracks for gapless playback</span>
            </div>
            <input type="checkbox" className="w-4 h-4 opacity-60 cursor-not-allowed" defaultChecked disabled />
          </div>
        </div>
      </PipelineStage>

      {/* Stage 5: Audio Output */}
      <PipelineStage
        title="Audio Output"
        description="Select your audio driver backend and output device for playback"
        isLast
      >
        <div className="space-y-6">
          <div>
            <label className="block text-sm font-medium mb-2">Backend</label>
            <div className="space-y-2">
              <label className="flex items-center gap-3 p-3 rounded-lg border cursor-not-allowed opacity-60">
                <input type="radio" name="backend" checked readOnly disabled />
                <div>
                  <span className="font-medium">Default (System Audio)</span>
                  <p className="text-xs text-muted-foreground">Standard system audio output</p>
                </div>
              </label>
              <label className="flex items-center gap-3 p-3 rounded-lg border cursor-not-allowed opacity-40">
                <input type="radio" name="backend" disabled />
                <div>
                  <span className="font-medium">ASIO</span>
                  <p className="text-xs text-muted-foreground">Low-latency Windows audio (desktop only)</p>
                </div>
              </label>
              <label className="flex items-center gap-3 p-3 rounded-lg border cursor-not-allowed opacity-40">
                <input type="radio" name="backend" disabled />
                <div>
                  <span className="font-medium">JACK</span>
                  <p className="text-xs text-muted-foreground">Professional audio on Linux (desktop only)</p>
                </div>
              </label>
            </div>
          </div>

          <div>
            <label className="block text-sm font-medium mb-2">Output Device</label>
            <select className="w-full max-w-xs px-3 py-2 border rounded-lg bg-background opacity-60 cursor-not-allowed" disabled>
              <option>Web Audio (Default)</option>
            </select>
            <p className="text-xs text-muted-foreground mt-1">
              Device selection available in desktop app
            </p>
          </div>
        </div>
      </PipelineStage>
    </div>
  )
}

// Pipeline Stage Component
interface PipelineStageProps {
  title: string
  description: string
  statusText?: string
  currentConfig?: string
  isOptional?: boolean
  isLast?: boolean
  children: React.ReactNode
}

function PipelineStage({
  title,
  description,
  statusText,
  currentConfig,
  isOptional,
  isLast,
  children,
}: PipelineStageProps) {
  return (
    <div className="relative">
      {/* Connector Line */}
      {!isLast && (
        <div className="absolute left-6 top-full h-4 w-0.5 bg-border" />
      )}

      <div className="bg-card rounded-xl border p-6">
        {/* Header */}
        <div className="flex items-start justify-between mb-4">
          <div>
            <div className="flex items-center gap-2">
              <h3 className="text-lg font-semibold">{title}</h3>
              {isOptional && (
                <span className="text-xs px-2 py-0.5 rounded-full bg-muted text-muted-foreground">
                  Optional
                </span>
              )}
            </div>
            <p className="text-sm text-muted-foreground mt-1">{description}</p>
          </div>
          {(statusText || currentConfig) && (
            <div className="text-right">
              {currentConfig && (
                <span className="text-sm font-medium">{currentConfig}</span>
              )}
              {statusText && (
                <span className="text-xs text-muted-foreground block">{statusText}</span>
              )}
            </div>
          )}
        </div>

        {/* Content */}
        {children}
      </div>
    </div>
  )
}

// Shortcuts Settings - matches desktop ShortcutsSettings structure
function ShortcutsSettings() {
  const shortcuts = [
    { action: 'play_pause', label: 'Play / Pause', keys: ['space'] },
    { action: 'next', label: 'Next Track', keys: ['mod', 'right'] },
    { action: 'previous', label: 'Previous Track', keys: ['mod', 'left'] },
    { action: 'volume_up', label: 'Volume Up', keys: ['mod', 'up'] },
    { action: 'volume_down', label: 'Volume Down', keys: ['mod', 'down'] },
    { action: 'mute', label: 'Mute', keys: ['mod', 'm'] },
    { action: 'toggle_shuffle', label: 'Toggle Shuffle', keys: ['mod', 's'] },
    { action: 'toggle_repeat', label: 'Toggle Repeat', keys: ['mod', 'r'] },
  ]

  return (
    <div className="max-w-2xl">
      <div className="mb-6">
        <h2 className="text-2xl font-semibold mb-2">Shortcuts</h2>
        <p className="text-sm text-muted-foreground">
          Configure keyboard shortcuts for playback control. Shortcuts only work when the app is focused and are disabled when typing in text fields.
        </p>
      </div>

      <div className="bg-card rounded-lg border border-border p-4 mb-6">
        {shortcuts.map((shortcut, index) => (
          <div
            key={shortcut.action}
            className={`flex items-center justify-between py-3 ${
              index !== shortcuts.length - 1 ? 'border-b border-border' : ''
            }`}
          >
            <span className="text-sm font-medium">{shortcut.label}</span>
            <button
              className="min-w-[120px] px-3 py-1.5 rounded-md text-sm bg-muted cursor-not-allowed opacity-70"
              disabled
            >
              <Kbd keys={shortcut.keys} size="sm" />
            </button>
          </div>
        ))}
      </div>

      <button
        className="px-4 py-2 border border-border rounded-lg opacity-50 cursor-not-allowed text-sm"
        disabled
      >
        Reset to Defaults
      </button>
      <p className="text-xs text-muted-foreground mt-2">
        Custom shortcuts available in desktop app
      </p>
    </div>
  )
}

// About Settings - matches desktop structure
function AboutSettings() {
  return (
    <div className="space-y-8">
      <section>
        <div className="flex items-center gap-4 mb-4">
          <div className="w-14 h-14 bg-primary/10 rounded-xl flex items-center justify-center">
            <Volume2 className="w-7 h-7 text-primary" />
          </div>
          <div>
            <h3 className="text-lg font-semibold">Soul Player</h3>
            <p className="text-sm text-muted-foreground">
              Version 0.1.0 (Demo)
            </p>
          </div>
        </div>
        <p className="text-sm text-muted-foreground">
          A local-first music player with high-quality audio processing pipeline. Download the desktop app for the full experience.
        </p>
      </section>

      <section>
        <h2 className="text-sm font-medium mb-3">Features</h2>
        <ul className="space-y-2 text-sm text-muted-foreground">
          <li className="flex items-center gap-2">
            <span className="text-green-500">✓</span>
            Symphonia-powered audio decoding (FLAC, MP3, WAV, AAC, OGG)
          </li>
          <li className="flex items-center gap-2">
            <span className="text-green-500">✓</span>
            High-quality DSP with EQ, compressor, and limiter
          </li>
          <li className="flex items-center gap-2">
            <span className="text-green-500">✓</span>
            Professional-grade resampling (r8brain algorithm)
          </li>
          <li className="flex items-center gap-2">
            <span className="text-green-500">✓</span>
            Gapless playback with crossfade support
          </li>
          <li className="flex items-center gap-2">
            <span className="text-green-500">✓</span>
            ReplayGain and EBU R128 volume leveling
          </li>
          <li className="flex items-center gap-2">
            <span className="text-green-500">✓</span>
            ASIO and JACK support for low-latency audio
          </li>
        </ul>
      </section>

      <section>
        <h2 className="text-sm font-medium mb-3">Links</h2>
        <div className="space-y-2">
          <a
            href={GITHUB_REPO}
            target="_blank"
            rel="noopener noreferrer"
            className="block text-sm text-primary hover:underline"
          >
            GitHub Repository
          </a>
          <a
            href="https://soulplayer.app"
            target="_blank"
            rel="noopener noreferrer"
            className="block text-sm text-primary hover:underline"
          >
            Website
          </a>
        </div>
      </section>
    </div>
  )
}
