/**
 * Sources modal content for demo
 */

'use client'

import { Folder, HardDrive, Server, Info } from 'lucide-react'

export function SourcesModalContent() {
  return (
    <div className="space-y-6">
      {/* Local Sources */}
      <section>
        <div className="flex items-center gap-2 mb-3">
          <Folder className="w-5 h-5 text-primary" />
          <h3 className="text-base font-medium">Local Music Folders</h3>
        </div>
        <div className="space-y-2 ml-7">
          <div className="flex items-center justify-between p-3 rounded-md border bg-muted/30">
            <div className="flex items-center gap-3">
              <HardDrive className="w-4 h-4 text-muted-foreground" />
              <div>
                <p className="text-sm font-medium">Demo Library</p>
                <p className="text-xs text-muted-foreground">/demo-audio • 2 tracks</p>
              </div>
            </div>
            <button
              className="px-3 py-1 text-xs rounded-md border hover:bg-accent transition-colors"
              disabled
            >
              Remove
            </button>
          </div>

          <button
            className="w-full py-2 px-4 rounded-md border border-dashed border-muted-foreground/30 text-sm text-muted-foreground hover:bg-muted/20 transition-colors"
            disabled
          >
            + Add Music Folder
          </button>
        </div>
      </section>

      {/* Remote Sources */}
      <section>
        <div className="flex items-center gap-2 mb-3">
          <Server className="w-5 h-5 text-primary" />
          <h3 className="text-base font-medium">Streaming Servers</h3>
        </div>
        <div className="space-y-2 ml-7">
          <div className="p-4 rounded-md border border-dashed border-muted-foreground/30 text-center">
            <Server className="w-8 h-8 text-muted-foreground mx-auto mb-2" />
            <p className="text-sm text-muted-foreground">No streaming servers configured</p>
            <p className="text-xs text-muted-foreground mt-1">
              Connect to Soul Server for multi-device streaming
            </p>
          </div>

          <button
            className="w-full py-2 px-4 rounded-md border border-dashed border-muted-foreground/30 text-sm text-muted-foreground hover:bg-muted/20 transition-colors"
            disabled
          >
            + Connect to Server
          </button>
        </div>
      </section>

      {/* Import Options */}
      <section>
        <div className="flex items-center gap-2 mb-3">
          <Info className="w-5 h-5 text-primary" />
          <h3 className="text-base font-medium">Import Settings</h3>
        </div>
        <div className="space-y-3 ml-7">
          <div className="flex items-center justify-between">
            <label className="text-sm text-muted-foreground">Watch for Changes</label>
            <div className="flex items-center gap-2">
              <input type="checkbox" defaultChecked disabled className="rounded" />
              <span className="text-xs text-muted-foreground">Enabled</span>
            </div>
          </div>
          <div className="flex items-center justify-between">
            <label className="text-sm text-muted-foreground">Auto-Import on Startup</label>
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
            This is a demo - source management is not functional
          </p>
        </div>
      </section>
    </div>
  )
}
