import { useEffect, useState } from 'react';

export type Platform = 'desktop' | 'mobile' | 'unknown';

export function usePlatform(): Platform {
  const [platform, setPlatform] = useState<Platform>('unknown');

  useEffect(() => {
    // Detect platform from Tauri
    const detectPlatform = async () => {
      try {
        // Try to import Tauri platform detection
        const { platform: getPlatform } = await import('@tauri-apps/plugin-os');
        const platformName = await getPlatform();

        if (platformName === 'ios' || platformName === 'android') {
          setPlatform('mobile');
        } else {
          setPlatform('desktop');
        }
      } catch {
        // Fallback: Check window dimensions
        const isMobile = window.matchMedia('(max-width: 768px)').matches;
        setPlatform(isMobile ? 'mobile' : 'desktop');
      }
    };

    detectPlatform();
  }, []);

  return platform;
}
