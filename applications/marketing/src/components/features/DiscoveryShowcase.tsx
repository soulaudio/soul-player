import { Database, Music, Search, Fingerprint } from 'lucide-react';
import { FadeIn } from '../animations/FadeIn';
import { ScrollRotate3D } from '../animations/ScrollRotate3D';
import { useTranslation } from 'react-i18next';

export function DiscoveryShowcase() {
  const { t } = useTranslation();

  return (
    <section className="py-24 px-6">
      <div className="max-w-7xl mx-auto">
        <div className="grid lg:grid-cols-2 gap-12 items-center">
          <FadeIn direction="left">
            <div>
              <h2 className="text-4xl font-bold mb-6">
                {t('features.discovery.title', 'Automatic Metadata Discovery')}
              </h2>
              <p className="text-xl text-muted-foreground mb-8">
                {t('features.discovery.description', 'Enrich your library with accurate metadata from multiple trusted sources.')}
              </p>

              <div className="space-y-6">
                <div className="flex gap-4">
                  <div className="flex-shrink-0 w-12 h-12 rounded-lg bg-primary/10 flex items-center justify-center">
                    <Database className="w-6 h-6 text-primary" />
                  </div>
                  <div>
                    <h3 className="font-semibold mb-2">
                      {t('features.discovery.musicbrainz.title', 'MusicBrainz Integration')}
                    </h3>
                    <p className="text-muted-foreground">
                      {t('features.discovery.musicbrainz.description', 'Access the world\'s largest open music encyclopedia for comprehensive metadata.')}
                    </p>
                  </div>
                </div>

                <div className="flex gap-4">
                  <div className="flex-shrink-0 w-12 h-12 rounded-lg bg-primary/10 flex items-center justify-center">
                    <Fingerprint className="w-6 h-6 text-primary" />
                  </div>
                  <div>
                    <h3 className="font-semibold mb-2">
                      {t('features.discovery.acoustid.title', 'AcoustID Fingerprinting')}
                    </h3>
                    <p className="text-muted-foreground">
                      {t('features.discovery.acoustid.description', 'Identify tracks using audio fingerprinting technology, even with missing tags.')}
                    </p>
                  </div>
                </div>

                <div className="flex gap-4">
                  <div className="flex-shrink-0 w-12 h-12 rounded-lg bg-primary/10 flex items-center justify-center">
                    <Music className="w-6 h-6 text-primary" />
                  </div>
                  <div>
                    <h3 className="font-semibold mb-2">
                      {t('features.discovery.discogs.title', 'Discogs & Bandcamp')}
                    </h3>
                    <p className="text-muted-foreground">
                      {t('features.discovery.discogs.description', 'Pull release information from Discogs and Bandcamp for independent releases.')}
                    </p>
                  </div>
                </div>

                <div className="flex gap-4">
                  <div className="flex-shrink-0 w-12 h-12 rounded-lg bg-primary/10 flex items-center justify-center">
                    <Search className="w-6 h-6 text-primary" />
                  </div>
                  <div>
                    <h3 className="font-semibold mb-2">
                      {t('features.discovery.smart.title', 'Smart Matching')}
                    </h3>
                    <p className="text-muted-foreground">
                      {t('features.discovery.smart.description', 'Intelligent algorithms match your files with the most accurate metadata available.')}
                    </p>
                  </div>
                </div>
              </div>
            </div>
          </FadeIn>

          <FadeIn direction="right" delay={0.2}>
            <ScrollRotate3D>
              <div className="relative">
                <div className="bg-gradient-to-br from-primary/20 to-accent/20 rounded-2xl p-8 backdrop-blur-sm border border-border">
                  <div className="space-y-4">
                    {/* Service Cards */}
                    <div className="bg-background/80 rounded-lg p-4 border border-border">
                      <div className="flex items-center gap-3 mb-2">
                        <Database className="w-5 h-5 text-primary" />
                        <span className="font-semibold">MusicBrainz</span>
                      </div>
                      <div className="text-sm text-muted-foreground">
                        {t('features.discovery.status.syncing', 'Syncing metadata...')}
                      </div>
                      <div className="mt-2 h-1 bg-primary/20 rounded-full overflow-hidden">
                        <div className="h-full bg-primary rounded-full w-3/4" />
                      </div>
                    </div>

                    <div className="bg-background/80 rounded-lg p-4 border border-border">
                      <div className="flex items-center gap-3 mb-2">
                        <Fingerprint className="w-5 h-5 text-primary" />
                        <span className="font-semibold">AcoustID</span>
                      </div>
                      <div className="text-sm text-muted-foreground">
                        {t('features.discovery.status.fingerprinting', 'Fingerprinting audio...')}
                      </div>
                      <div className="mt-2 h-1 bg-primary/20 rounded-full overflow-hidden">
                        <div className="h-full bg-primary rounded-full w-1/2" />
                      </div>
                    </div>

                    <div className="bg-background/80 rounded-lg p-4 border border-border">
                      <div className="flex items-center gap-3 mb-2">
                        <Music className="w-5 h-5 text-primary" />
                        <span className="font-semibold">Discogs</span>
                      </div>
                      <div className="text-sm text-green-500">
                        {t('features.discovery.status.complete', '✓ Metadata updated')}
                      </div>
                    </div>

                    <div className="bg-background/80 rounded-lg p-4 border border-border">
                      <div className="flex items-center gap-3 mb-2">
                        <Search className="w-5 h-5 text-primary" />
                        <span className="font-semibold">Bandcamp</span>
                      </div>
                      <div className="text-sm text-green-500">
                        {t('features.discovery.status.complete', '✓ Metadata updated')}
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
