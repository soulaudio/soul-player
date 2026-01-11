/**
 * Platform context - provides platform-awareness for conditional rendering
 * Desktop: isDesktop = true
 * Marketing demo / Web: isDesktop = false
 */

import { createContext, useContext, ReactNode } from 'react';

export type PlatformType = 'desktop' | 'web' | 'mobile';

export interface PlatformContextValue {
  platform: PlatformType;
  isDesktop: boolean;
  isWeb: boolean;
  isMobile: boolean;
  // Feature flags based on platform
  features: {
    // Library features
    canDeleteTracks: boolean;
    canCreatePlaylists: boolean;
    hasFilters: boolean;
    hasHealthCheck: boolean;
    hasVirtualization: boolean;
    hasTrackMenu: boolean;
    hasPlaybackContext: boolean;
    // Settings features
    hasLibrarySettings: boolean;
    hasAudioSettings: boolean;
    hasShortcutSettings: boolean;
    hasUpdateSettings: boolean;
    hasLanguageSettings: boolean;
    hasThemeImportExport: boolean;
    // Audio features
    hasRealAudioDevices: boolean;
    hasRealDeviceSelection: boolean;
  };
}

const defaultFeatures = {
  canDeleteTracks: false,
  canCreatePlaylists: false,
  hasFilters: false,
  hasHealthCheck: false,
  hasVirtualization: false,
  hasTrackMenu: false,
  hasPlaybackContext: false,
  hasLibrarySettings: false,
  hasAudioSettings: false,
  hasShortcutSettings: false,
  hasUpdateSettings: false,
  hasLanguageSettings: false,
  hasThemeImportExport: false,
  hasRealAudioDevices: false,
  hasRealDeviceSelection: false,
};

const PlatformContext = createContext<PlatformContextValue>({
  platform: 'web',
  isDesktop: false,
  isWeb: true,
  isMobile: false,
  features: defaultFeatures,
});

export function usePlatform(): PlatformContextValue {
  return useContext(PlatformContext);
}

export function useIsDesktop(): boolean {
  return useContext(PlatformContext).isDesktop;
}

export function useFeatures(): PlatformContextValue['features'] {
  return useContext(PlatformContext).features;
}

interface PlatformProviderProps {
  children: ReactNode;
  platform: PlatformType;
  features?: Partial<PlatformContextValue['features']>;
}

export function PlatformProvider({ children, platform, features }: PlatformProviderProps) {
  const value: PlatformContextValue = {
    platform,
    isDesktop: platform === 'desktop',
    isWeb: platform === 'web',
    isMobile: platform === 'mobile',
    features: {
      ...defaultFeatures,
      ...features,
    },
  };

  return (
    <PlatformContext.Provider value={value}>
      {children}
    </PlatformContext.Provider>
  );
}

// Conditional rendering components

interface DesktopOnlyProps {
  children: ReactNode;
  fallback?: ReactNode;
}

/**
 * Renders children only on desktop platform
 */
export function DesktopOnly({ children, fallback = null }: DesktopOnlyProps) {
  const { isDesktop } = usePlatform();
  return isDesktop ? <>{children}</> : <>{fallback}</>;
}

interface WebOnlyProps {
  children: ReactNode;
  fallback?: ReactNode;
}

/**
 * Renders children only on web platform (marketing demo)
 */
export function WebOnly({ children, fallback = null }: WebOnlyProps) {
  const { isWeb } = usePlatform();
  return isWeb ? <>{children}</> : <>{fallback}</>;
}

interface FeatureGateProps {
  feature: keyof PlatformContextValue['features'];
  children: ReactNode;
  fallback?: ReactNode;
}

/**
 * Renders children only if the specified feature is enabled
 */
export function FeatureGate({ feature, children, fallback = null }: FeatureGateProps) {
  const features = useFeatures();
  return features[feature] ? <>{children}</> : <>{fallback}</>;
}
