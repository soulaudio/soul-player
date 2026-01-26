import { Smartphone, Tablet, Cloud } from 'lucide-react';
import { FadeIn } from '../animations/FadeIn';
import { ScrollRotate3D } from '../animations/ScrollRotate3D';

export function MobileShowcase() {
  return (
    <section className="py-24 px-6 bg-muted/30">
      <div className="max-w-7xl mx-auto">
        <div className="grid lg:grid-cols-2 gap-16 items-center">
          {/* Left: Content */}
          <FadeIn>
            <div className="space-y-8">
              <div>
                <h2 className="text-4xl font-bold mb-4">
                  Your Music, Everywhere
                </h2>
                <p className="text-xl text-muted-foreground">
                  Take your entire library on the go. Soul Player syncs seamlessly across all your devices, so your music is always within reach.
                </p>
              </div>

              <div className="space-y-6">
                <div className="flex gap-4">
                  <div className="flex-shrink-0 w-12 h-12 rounded-lg bg-primary/10 flex items-center justify-center">
                    <Smartphone className="w-6 h-6 text-primary" />
                  </div>
                  <div>
                    <h3 className="font-semibold mb-2">iOS & Android Apps</h3>
                    <p className="text-muted-foreground">
                      Native apps designed for your phone. Fast, fluid, and familiar.
                    </p>
                  </div>
                </div>

                <div className="flex gap-4">
                  <div className="flex-shrink-0 w-12 h-12 rounded-lg bg-primary/10 flex items-center justify-center">
                    <Cloud className="w-6 h-6 text-primary" />
                  </div>
                  <div>
                    <h3 className="font-semibold mb-2">Seamless Sync</h3>
                    <p className="text-muted-foreground">
                      Playlists, play counts, and favorites stay in sync across all devices automatically.
                    </p>
                  </div>
                </div>

                <div className="flex gap-4">
                  <div className="flex-shrink-0 w-12 h-12 rounded-lg bg-primary/10 flex items-center justify-center">
                    <Tablet className="w-6 h-6 text-primary" />
                  </div>
                  <div>
                    <h3 className="font-semibold mb-2">Offline Playback</h3>
                    <p className="text-muted-foreground">
                      Download your favorites for offline listening. No connection required.
                    </p>
                  </div>
                </div>
              </div>
            </div>
          </FadeIn>

          {/* Right: Visual Mockup - Device Frames */}
          <FadeIn delay={0.2}>
            <ScrollRotate3D>
              <div className="relative">
                <div className="flex gap-8 items-end justify-center">
                  {/* Phone Frame */}
                  <div className="relative">
                    <div className="w-64 h-[32rem] bg-card border-4 border-border rounded-[2.5rem] shadow-2xl overflow-hidden">
                      {/* Phone Notch */}
                      <div className="h-6 bg-background flex items-center justify-center">
                        <div className="w-32 h-5 bg-card rounded-b-2xl" />
                      </div>

                      {/* Phone Content */}
                      <div className="p-4 space-y-4">
                        <div className="aspect-square bg-gradient-to-br from-primary/20 to-primary/5 rounded-xl flex items-center justify-center">
                          <div className="w-16 h-16 bg-primary/30 rounded-full" />
                        </div>
                        <div className="space-y-2">
                          <div className="h-4 bg-muted rounded w-3/4" />
                          <div className="h-3 bg-muted/60 rounded w-1/2" />
                        </div>
                        <div className="space-y-2">
                          {[...Array(4)].map((_, i) => (
                            <div key={i} className="flex gap-3 items-center">
                              <div className="w-12 h-12 bg-muted/40 rounded-lg flex-shrink-0" />
                              <div className="flex-1 space-y-1">
                                <div className="h-3 bg-muted/60 rounded" />
                                <div className="h-2 bg-muted/40 rounded w-2/3" />
                              </div>
                            </div>
                          ))}
                        </div>
                      </div>
                    </div>
                  </div>

                  {/* Tablet Frame */}
                  <div className="relative">
                    <div className="w-80 h-60 bg-card border-4 border-border rounded-3xl shadow-2xl overflow-hidden">
                      {/* Tablet Content */}
                      <div className="p-6 h-full">
                        <div className="grid grid-cols-3 gap-4 h-full">
                          {[...Array(6)].map((_, i) => (
                            <div key={i} className="space-y-2">
                              <div className="aspect-square bg-gradient-to-br from-primary/20 to-primary/5 rounded-lg" />
                              <div className="h-2 bg-muted/60 rounded" />
                              <div className="h-2 bg-muted/40 rounded w-2/3" />
                            </div>
                          ))}
                        </div>
                      </div>
                    </div>
                  </div>
                </div>

                {/* Sync Indicator */}
                <div className="absolute -top-4 left-1/2 -translate-x-1/2 bg-primary text-primary-foreground px-4 py-2 rounded-full text-sm font-medium shadow-lg flex items-center gap-2">
                  <Cloud className="w-4 h-4" />
                  Synced
                </div>
              </div>
            </ScrollRotate3D>
          </FadeIn>
        </div>
      </div>
    </section>
  );
}
