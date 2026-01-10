import type { Theme } from '../types';

/**
 * Ocean theme for Soul Player
 * Deep sea palette transitioning from surface azure to abyss depths
 * Calm and immersive, inspired by ocean layers at different depths
 */
export const oceanTheme: Theme = {
  id: 'ocean',
  name: 'Ocean',
  version: '1.0.0',
  author: 'Soul Player Team',
  description: 'Deep sea theme with azure surfaces and mysterious depths',
  isBuiltIn: true,

  colors: {
    // Base: Deep ocean floor
    background: '210 45% 8%',
    foreground: '195 30% 88%',

    // Cards: Mid-depth layer
    card: '208 40% 12%',
    'card-foreground': '195 30% 88%',

    // Popovers: Similar to cards
    popover: '208 40% 12%',
    'popover-foreground': '195 30% 88%',

    // Primary: Bioluminescent cyan (life in the deep)
    primary: '185 70% 50%',
    'primary-foreground': '210 45% 8%',

    // Secondary: Twilight zone blue
    secondary: '215 35% 22%',
    'secondary-foreground': '195 30% 88%',

    // Muted: Abyssal shadow
    muted: '210 30% 14%',
    'muted-foreground': '200 20% 50%',

    // Accent: Coral reef warmth (underwater life)
    accent: '15 60% 55%',
    'accent-foreground': '195 30% 92%',

    // Destructive: Deep sea warning red
    destructive: '355 55% 45%',
    'destructive-foreground': '195 30% 92%',

    // UI elements
    border: '210 30% 18%',
    input: '210 30% 18%',
    ring: '185 70% 50%',
  },

  gradients: {
    hero: 'linear-gradient(135deg, hsl(210 45% 8%) 0%, hsl(208 40% 16%) 100%)',
    player: 'linear-gradient(to bottom, hsl(210 45% 8%) 0%, hsl(210 30% 12%) 100%)',
    sidebar: 'linear-gradient(to right, hsl(210 30% 14%) 0%, hsl(210 45% 8%) 100%)',
    waveform: 'linear-gradient(90deg, hsl(185 70% 50%) 0%, hsl(15 60% 55%) 50%, hsl(185 70% 50%) 100%)',
  },

  typography: {
    fontFamily: {
      sans: ['Inter', 'system-ui', 'sans-serif'],
      mono: ['JetBrains Mono', 'Consolas', 'monospace'],
    },
    fontSize: {
      base: '16px',
    },
  },
};
