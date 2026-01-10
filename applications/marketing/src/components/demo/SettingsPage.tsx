/**
 * Demo Settings Page - matches desktop UI structure
 * Uses shared ThemePicker component for theme selection
 */
'use client'

import { useState } from 'react'
import { ThemePicker } from '@soul-player/shared/theme'
import { Kbd } from '@soul-player/shared'

type SettingsTab = 'general' | 'audio' | 'shortcuts' | 'about'

export function SettingsPage() {
  const [activeTab, setActiveTab] = useState<SettingsTab>('general')

  const tabs: { id: SettingsTab; label: string }[] = [
    { id: 'general', label: 'General' },
    { id: 'audio', label: 'Audio' },
    { id: 'shortcuts', label: 'Shortcuts' },
    { id: 'about', label: 'About' },
  ]

  return (
    <div className="h-full flex flex-col">
      {/* Tabs Navigation - matches desktop */}
      <div className="border-b border-border px-6">
        <div className="flex space-x-8">
          {tabs.map((tab) => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              className={`
                py-4 px-2 border-b-2 transition-colors font-medium text-sm
                ${
                  activeTab === tab.id
                    ? 'border-primary text-foreground'
                    : 'border-transparent text-muted-foreground hover:text-foreground'
                }
              `}
            >
              {tab.label}
            </button>
          ))}
        </div>
      </div>

      {/* Tab Content */}
      <div className="flex-1 overflow-y-auto p-8">
        {activeTab === 'general' && <GeneralSettings />}
        {activeTab === 'audio' && <AudioSettings />}
        {activeTab === 'shortcuts' && <ShortcutsSettings />}
        {activeTab === 'about' && <AboutSettings />}
      </div>
    </div>
  )
}

// General Settings Tab Content - matches desktop structure
function GeneralSettings() {
  return (
    <div className="max-w-4xl space-y-8">
      {/* Appearance Section */}
      <section>
        <h2 className="text-2xl font-semibold mb-4">Appearance</h2>

        <div className="mb-6">
          <label className="block text-sm font-medium mb-2">Theme</label>
          <ThemePicker
            showImportExport={false}
            showAccessibilityInfo={true}
          />
        </div>

        <div className="mb-6">
          <label className="block text-sm font-medium mb-2">Language</label>
          <select
            className="w-full max-w-xs px-3 py-2 border rounded-lg bg-background"
            defaultValue="en-US"
          >
            <option value="en-US">English (US)</option>
            <option value="de">Deutsch</option>
            <option value="ja">日本語</option>
          </select>
          <p className="text-xs text-muted-foreground mt-1">
            Language selection available in full app
          </p>
        </div>

        <div>
          <label className="flex items-start space-x-3 cursor-pointer">
            <input
              type="checkbox"
              defaultChecked={false}
              className="w-4 h-4 mt-0.5"
            />
            <div>
              <span className="text-sm font-medium block">Show keyboard shortcuts</span>
              <p className="text-xs text-muted-foreground mt-1">
                Display keyboard shortcuts in tooltips and UI elements. For example: <Kbd keys={['mod', 'k']} size="sm" />
              </p>
            </div>
          </label>
        </div>
      </section>

      {/* Updates Section */}
      <section>
        <h2 className="text-2xl font-semibold mb-4">Updates</h2>
        <div className="space-y-4">
          <label className="flex items-center space-x-3">
            <input
              type="checkbox"
              defaultChecked={true}
              className="w-4 h-4"
            />
            <span className="text-sm">Automatically check for updates</span>
          </label>

          <label className="flex items-center space-x-3">
            <input
              type="checkbox"
              defaultChecked={false}
              className="w-4 h-4"
            />
            <span className="text-sm">Install updates silently in background</span>
          </label>

          <button
            className="px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 disabled:opacity-50"
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

// Audio Settings Tab Content - demo version showing available features
function AudioSettings() {
  return (
    <div className="max-w-4xl space-y-8">
      <div>
        <h1 className="text-3xl font-bold mb-2">Audio</h1>
        <p className="text-muted-foreground">
          Configure high-quality audio processing pipeline
        </p>
      </div>

      {/* Pipeline Visualization Preview */}
      <div className="bg-card border border-border rounded-lg p-6">
        <div className="flex items-center gap-3 mb-4">
          <div className="flex items-center gap-2">
            <div className="w-3 h-3 rounded-full bg-green-500"></div>
            <span className="text-sm font-medium">Audio Pipeline</span>
          </div>
        </div>
        <div className="flex items-center gap-2 text-sm text-muted-foreground">
          <span className="px-2 py-1 rounded bg-muted">Source</span>
          <span>→</span>
          <span className="px-2 py-1 rounded bg-muted">Decode</span>
          <span>→</span>
          <span className="px-2 py-1 rounded bg-muted">DSP</span>
          <span>→</span>
          <span className="px-2 py-1 rounded bg-muted">Upsample</span>
          <span>→</span>
          <span className="px-2 py-1 rounded bg-muted">Output</span>
        </div>
      </div>

      {/* Audio Driver Section */}
      <section className="space-y-4">
        <div>
          <h2 className="text-xl font-semibold mb-1">Audio Driver</h2>
          <p className="text-sm text-muted-foreground">
            Select audio backend and output device
          </p>
        </div>

        <div className="bg-card border border-border rounded-lg p-6 space-y-6">
          <div>
            <label className="block text-sm font-medium mb-2">Backend</label>
            <select className="w-full max-w-xs px-3 py-2 border rounded-lg bg-background">
              <option>Default (System Audio)</option>
              <option disabled>ASIO (Windows)</option>
              <option disabled>JACK (Linux)</option>
            </select>
            <p className="text-xs text-muted-foreground mt-1">
              Low-latency backends available in desktop app
            </p>
          </div>

          <div>
            <label className="block text-sm font-medium mb-2">Output Device</label>
            <select className="w-full max-w-xs px-3 py-2 border rounded-lg bg-background">
              <option>Web Audio (Default)</option>
            </select>
          </div>
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

        <div className="bg-card border border-border rounded-lg p-6">
          <div className="flex items-center justify-between mb-4">
            <span className="font-medium">Enable DSP Processing</span>
            <input type="checkbox" className="w-4 h-4" defaultChecked={false} />
          </div>
          <div className="grid grid-cols-2 gap-4">
            <div className="p-3 border border-dashed rounded-lg text-center text-sm text-muted-foreground">
              EQ Slot 1
            </div>
            <div className="p-3 border border-dashed rounded-lg text-center text-sm text-muted-foreground">
              EQ Slot 2
            </div>
            <div className="p-3 border border-dashed rounded-lg text-center text-sm text-muted-foreground">
              Compressor Slot
            </div>
            <div className="p-3 border border-dashed rounded-lg text-center text-sm text-muted-foreground">
              Limiter Slot
            </div>
          </div>
          <p className="text-xs text-muted-foreground mt-4">
            Full DSP configuration available in desktop app
          </p>
        </div>
      </section>

      {/* Upsampling Section */}
      <section className="space-y-4">
        <div>
          <h2 className="text-xl font-semibold mb-1">Upsampling / Resampling</h2>
          <p className="text-sm text-muted-foreground">
            High-quality sample rate conversion using r8brain algorithm
          </p>
        </div>

        <div className="bg-card border border-border rounded-lg p-6">
          <div className="space-y-4">
            <div>
              <label className="block text-sm font-medium mb-2">Quality</label>
              <select className="w-full max-w-xs px-3 py-2 border rounded-lg bg-background">
                <option>High (Recommended)</option>
                <option>Fast</option>
                <option>Balanced</option>
                <option>Maximum</option>
                <option>Disabled</option>
              </select>
            </div>
            <div>
              <label className="block text-sm font-medium mb-2">Target Sample Rate</label>
              <select className="w-full max-w-xs px-3 py-2 border rounded-lg bg-background">
                <option>Auto (Match Device)</option>
                <option>96 kHz</option>
                <option>192 kHz</option>
              </select>
            </div>
          </div>
        </div>
      </section>

      {/* Volume Leveling Section */}
      <section className="space-y-4">
        <div>
          <h2 className="text-xl font-semibold mb-1">Volume Leveling</h2>
          <p className="text-sm text-muted-foreground">
            Automatic loudness normalization (ReplayGain / EBU R128)
          </p>
        </div>

        <div className="bg-card border border-border rounded-lg p-6">
          <div>
            <label className="block text-sm font-medium mb-2">Mode</label>
            <select className="w-full max-w-xs px-3 py-2 border rounded-lg bg-background">
              <option>Disabled</option>
              <option>ReplayGain (Track)</option>
              <option>ReplayGain (Album)</option>
              <option>EBU R128</option>
            </select>
          </div>
        </div>
      </section>
    </div>
  )
}

// Shortcuts Settings Tab Content
function ShortcutsSettings() {
  return (
    <div className="max-w-4xl">
      <h2 className="text-3xl font-bold mb-6">Shortcuts</h2>
      <p className="text-sm text-muted-foreground mb-6">
        Configure global keyboard shortcuts for playback control.
      </p>

      <div className="bg-card border border-border rounded-lg p-6 space-y-4">
        <div className="flex items-center justify-between py-2">
          <span className="text-sm">Play / Pause</span>
          <Kbd keys={['space']} />
        </div>
        <div className="flex items-center justify-between py-2 border-t border-border">
          <span className="text-sm">Next Track</span>
          <Kbd keys={['mod', 'right']} />
        </div>
        <div className="flex items-center justify-between py-2 border-t border-border">
          <span className="text-sm">Previous Track</span>
          <Kbd keys={['mod', 'left']} />
        </div>
        <div className="flex items-center justify-between py-2 border-t border-border">
          <span className="text-sm">Search</span>
          <Kbd keys={['mod', 'k']} />
        </div>
        <div className="flex items-center justify-between py-2 border-t border-border">
          <span className="text-sm">Toggle Shuffle</span>
          <Kbd keys={['mod', 's']} />
        </div>
        <div className="flex items-center justify-between py-2 border-t border-border">
          <span className="text-sm">Toggle Repeat</span>
          <Kbd keys={['mod', 'r']} />
        </div>
      </div>

      <button
        className="mt-6 px-4 py-2 border rounded-lg hover:bg-muted disabled:opacity-50"
        disabled
      >
        Configure Shortcuts
      </button>
      <p className="text-xs text-muted-foreground mt-2">
        Custom shortcuts available in desktop app
      </p>
    </div>
  )
}

// About Settings Tab Content
function AboutSettings() {
  return (
    <div className="max-w-4xl">
      <h2 className="text-3xl font-bold mb-6">About</h2>
      <div className="bg-muted/40 rounded-lg p-6 space-y-3">
        <p className="text-lg">
          <span className="font-semibold">Soul Player</span> - Local-first music player
        </p>
        <p className="text-sm text-muted-foreground">
          Version 0.1.0 (Demo)
        </p>
        <p className="text-sm text-muted-foreground">
          High-quality audio playback with professional audio processing pipeline
        </p>
      </div>

      <div className="mt-8 space-y-4">
        <h3 className="text-lg font-semibold">Features</h3>
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
      </div>

      <div className="mt-8 pt-6 border-t">
        <p className="text-xs text-muted-foreground">
          This is a demo version showcasing Soul Player's interface.
          Download the full desktop app for all features.
        </p>
      </div>
    </div>
  )
}
