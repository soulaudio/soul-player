# Soul Player Shared

Shared React components, stores, hooks, and utilities.

## Usage

```typescript
import { usePlayerStore, commands, Track } from '@soul-player/shared';

function MyComponent() {
  const { currentTrack, isPlaying } = usePlayerStore();

  const handlePlay = async (track: Track) => {
    await commands.playTrack(track.id);
  };

  return <div>{currentTrack?.title}</div>;
}
```

## Development

```bash
# From repository root
yarn workspace @soul-player/shared test          # Run tests
yarn workspace @soul-player/shared test:watch    # Watch mode
yarn workspace @soul-player/shared type-check    # TypeScript check
yarn workspace @soul-player/shared lint          # ESLint

# Or from applications/shared/
yarn test
yarn test:watch
yarn type-check
yarn lint
```

## Structure

```
src/
  components/    # UI components (shadcn/ui)
  stores/        # Zustand state
  hooks/         # React hooks
  lib/           # Utilities + Tauri commands
  types/         # TypeScript types
```
