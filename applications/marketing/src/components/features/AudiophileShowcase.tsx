import { Music, Zap, Radio } from 'lucide-react';
import { FadeIn } from '../animations/FadeIn';
import { ScrollRotate3D } from '../animations/ScrollRotate3D';

export function AudiophileShowcase() {
  return (
    <section className="py-24 px-6">
      <div className="max-w-7xl mx-auto">
        <div className="grid lg:grid-cols-2 gap-16 items-center">
          {/* Left: Content */}
          <FadeIn>
            <div className="space-y-8">
              <div>
                <h2 className="text-4xl font-bold mb-4">
                  Audiophile-Grade Quality
                </h2>
                <p className="text-xl text-muted-foreground">
                  Experience your music the way artists intended. Soul Player delivers bit-perfect playback with support for the highest resolution audio formats.
                </p>
              </div>

              <div className="space-y-6">
                <div className="flex gap-4">
                  <div className="flex-shrink-0 w-12 h-12 rounded-lg bg-primary/10 flex items-center justify-center">
                    <Music className="w-6 h-6 text-primary" />
                  </div>
                  <div>
                    <h3 className="font-semibold mb-2">Bit-Perfect Playback</h3>
                    <p className="text-muted-foreground">
                      Zero alteration to your audio files. Every bit arrives exactly as encoded.
                    </p>
                  </div>
                </div>

                <div className="flex gap-4">
                  <div className="flex-shrink-0 w-12 h-12 rounded-lg bg-primary/10 flex items-center justify-center">
                    <Zap className="w-6 h-6 text-primary" />
                  </div>
                  <div>
                    <h3 className="font-semibold mb-2">WASAPI/ASIO Exclusive Mode</h3>
                    <p className="text-muted-foreground">
                      Bypass system mixers for direct hardware access and minimal latency.
                    </p>
                  </div>
                </div>

                <div className="flex gap-4">
                  <div className="flex-shrink-0 w-12 h-12 rounded-lg bg-primary/10 flex items-center justify-center">
                    <Radio className="w-6 h-6 text-primary" />
                  </div>
                  <div>
                    <h3 className="font-semibold mb-2">Hi-Res Audio Support</h3>
                    <p className="text-muted-foreground">
                      DSD256, 32-bit/384kHz PCM, and everything in between. Your DAC's full potential, unleashed.
                    </p>
                  </div>
                </div>
              </div>
            </div>
          </FadeIn>

          {/* Right: Visual Mockup */}
          <FadeIn delay={0.2}>
            <ScrollRotate3D>
              <div className="relative">
                <div className="bg-card border border-border rounded-2xl p-8 shadow-2xl">
                  {/* Audio Specs Display */}
                  <div className="space-y-6">
                    <div className="text-center border-b border-border pb-4">
                      <div className="text-sm text-muted-foreground mb-1">Now Playing</div>
                      <div className="font-semibold">DSD256 • 11.2 MHz</div>
                    </div>

                    {/* Waveform Visualization */}
                    <div className="flex items-end justify-center gap-1 h-32">
                      {[...Array(32)].map((_, i) => {
                        const height = Math.sin(i * 0.5) * 40 + 60;
                        return (
                          <div
                            key={i}
                            className="w-2 bg-gradient-to-t from-primary to-primary/40 rounded-t"
                            style={{ height: `${height}%` }}
                          />
                        );
                      })}
                    </div>

                    {/* Audio Format Info */}
                    <div className="grid grid-cols-2 gap-4 pt-4 border-t border-border">
                      <div>
                        <div className="text-xs text-muted-foreground mb-1">Format</div>
                        <div className="text-sm font-medium">DSD256</div>
                      </div>
                      <div>
                        <div className="text-xs text-muted-foreground mb-1">Sample Rate</div>
                        <div className="text-sm font-medium">11.2 MHz</div>
                      </div>
                      <div>
                        <div className="text-xs text-muted-foreground mb-1">Bit Depth</div>
                        <div className="text-sm font-medium">1-bit</div>
                      </div>
                      <div>
                        <div className="text-xs text-muted-foreground mb-1">Output</div>
                        <div className="text-sm font-medium">ASIO</div>
                      </div>
                    </div>

                    {/* Additional Formats */}
                    <div className="pt-4 border-t border-border">
                      <div className="text-xs text-muted-foreground mb-3">Supported Formats</div>
                      <div className="flex flex-wrap gap-2">
                        {['DSD256', 'DSD128', '32/384', '24/192', 'FLAC', 'ALAC', 'WAV', 'AIFF'].map((format) => (
                          <span
                            key={format}
                            className="px-3 py-1 bg-primary/10 text-primary text-xs rounded-full font-medium"
                          >
                            {format}
                          </span>
                        ))}
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </ScrollRotate3D>
          </FadeIn>
        </div>
      </div>
    </section>
  );
}
