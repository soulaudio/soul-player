/**
 * Demo implementation of PlayerCommands context
 * Uses WebPlaybackProvider from shared package
 *
 * This is now a thin wrapper around WebPlaybackProvider,
 * which handles all the heavy lifting of WASM initialization,
 * event wiring, and command implementation.
 */

import { ReactNode } from 'react';
import { WebPlaybackProvider, type DemoStorage } from '@soul-player/shared';

interface DemoPlayerCommandsProviderProps {
  storage: DemoStorage;
  children: ReactNode;
}

/**
 * Demo-specific player commands provider
 * Delegates to WebPlaybackProvider with DemoStorage as data source
 */
export function DemoPlayerCommandsProvider({ storage, children }: DemoPlayerCommandsProviderProps) {
  return <WebPlaybackProvider storage={storage}>{children}</WebPlaybackProvider>;
}
