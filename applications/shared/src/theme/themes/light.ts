import type { Theme } from '../types';

/**
 * Light theme for Soul Player
 * Warm ivory/cream palette inspired by morning sunlight through paper
 * Comfortable, refined, and easy on the eyes
 */
export const lightTheme: Theme = {
  id: 'light',
  name: 'Light',
  version: '1.0.0',
  author: 'Soul Player Team',
  description: 'Warm and inviting theme with cream tones and subtle warmth',
  isBuiltIn: true,

  colors: {
    // Base: Warm ivory (not stark white)
    background: '40 30% 98%',
    foreground: '30 15% 15%',

    // Cards: Soft cream
    card: '45 25% 96%',
    'card-foreground': '30 15% 15%',

    // Popovers: Slightly elevated cream
    popover: '45 25% 96%',
    'popover-foreground': '30 15% 15%',

    // Primary: Rich espresso brown
    primary: '25 35% 25%',
    'primary-foreground': '40 30% 98%',

    // Secondary: Soft sand
    secondary: '35 20% 92%',
    'secondary-foreground': '30 15% 25%',

    // Muted: Light parchment
    muted: '40 18% 93%',
    'muted-foreground': '30 10% 50%',

    // Accent: Warm terracotta/rust
    accent: '18 45% 55%',
    'accent-foreground': '40 30% 98%',

    // Destructive: Warm coral red
    destructive: '5 65% 55%',
    'destructive-foreground': '40 30% 98%',

    // UI elements
    border: '35 15% 88%',
    input: '35 15% 88%',
    ring: '25 35% 25%',
  },

  gradients: {
    hero: 'linear-gradient(135deg, hsl(40 30% 98%) 0%, hsl(35 20% 92%) 100%)',
    player: 'linear-gradient(to bottom, hsl(45 25% 96%) 0%, hsl(40 18% 93%) 100%)',
    sidebar: 'linear-gradient(to right, hsl(40 18% 93%) 0%, hsl(40 30% 98%) 100%)',
    waveform: 'linear-gradient(90deg, hsl(25 35% 25%) 0%, hsl(18 45% 55%) 50%, hsl(25 35% 25%) 100%)',
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
