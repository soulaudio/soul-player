/**
 * Settings modal content for demo
 */

'use client'

import { Volume2, Music, Palette, Info } from 'lucide-react'

export function SettingsModalContent() {
  return (
    <div className="space-y-6">
      {/* Audio Settings */}
      <section>
        <div className="flex items-center gap-2 mb-3">
          <Volume2 className="w-5 h-5 text-primary" />
          <h3 className="text-base font-medium">Audio</h3>
        </div>
        <div className="space-y-3 ml-7">
          <div className="flex items-center justify-between">
            <label className="text-sm text-muted-foreground">Output Device</label>
            <select className="px-3 py-1.5 rounded-md border bg-background text-sm">
              <option>Default Audio Output</option>
              <option disabled>Speakers (Demo)</option>
              <option disabled>Headphones (Demo)</option>
            </select>
          </div>
          <div className="flex items-center justify-between">
            <label className="text-sm text-muted-foreground">Sample Rate</label>
            <select className="px-3 py-1.5 rounded-md border bg-background text-sm">
              <option>Auto (96 kHz)</option>
              <option disabled>44.1 kHz</option>
              <option disabled>48 kHz</option>
            </select>
          </div>
          <div className="flex items-center justify-between">
            <label className="text-sm text-muted-foreground">Gapless Playback</label>
            <div className="flex items-center gap-2">
              <input type="checkbox" defaultChecked disabled className="rounded" />
              <span className="text-xs text-muted-foreground">Enabled</span>
            </div>
          </div>
        </div>
      </section>

      {/* Playback Settings */}
      <section>
        <div className="flex items-center gap-2 mb-3">
          <Music className="w-5 h-5 text-primary" />
          <h3 className="text-base font-medium">Playback</h3>
        </div>
        <div className="space-y-3 ml-7">
          <div className="flex items-center justify-between">
            <label className="text-sm text-muted-foreground">Default Shuffle</label>
            <select className="px-3 py-1.5 rounded-md border bg-background text-sm">
              <option>Off</option>
              <option>Random</option>
              <option>Smart</option>
            </select>
          </div>
          <div className="flex items-center justify-between">
            <label className="text-sm text-muted-foreground">Default Repeat</label>
            <select className="px-3 py-1.5 rounded-md border bg-background text-sm">
              <option>Off</option>
              <option>All</option>
              <option>One</option>
            </select>
          </div>
          <div className="flex items-center justify-between">
            <label className="text-sm text-muted-foreground">History Size</label>
            <select className="px-3 py-1.5 rounded-md border bg-background text-sm">
              <option>50 tracks</option>
              <option disabled>25 tracks</option>
              <option disabled>100 tracks</option>
            </select>
          </div>
        </div>
      </section>

      {/* Appearance */}
      <section>
        <div className="flex items-center gap-2 mb-3">
          <Palette className="w-5 h-5 text-primary" />
          <h3 className="text-base font-medium">Appearance</h3>
        </div>
        <div className="space-y-3 ml-7">
          <div className="flex items-center justify-between">
            <label className="text-sm text-muted-foreground">Theme</label>
            <select className="px-3 py-1.5 rounded-md border bg-background text-sm">
              <option>Dark</option>
              <option>Light</option>
              <option>Ocean</option>
            </select>
          </div>
          <div className="flex items-center justify-between">
            <label className="text-sm text-muted-foreground">Compact Mode</label>
            <div className="flex items-center gap-2">
              <input type="checkbox" disabled className="rounded" />
              <span className="text-xs text-muted-foreground">Disabled</span>
            </div>
          </div>
        </div>
      </section>

      {/* Info */}
      <section className="pt-4 border-t">
        <div className="flex items-center gap-2 mb-2">
          <Info className="w-4 h-4 text-muted-foreground" />
          <p className="text-xs text-muted-foreground">
            This is a demo - settings are not saved
          </p>
        </div>
      </section>
    </div>
  )
}
