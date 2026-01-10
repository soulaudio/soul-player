import type { Theme } from '../types';

/**
 * Dark theme for Soul Player
 * Deep midnight palette with subtle violet undertones
 * Sophisticated and immersive, inspired by a starlit night sky
 */
export const darkTheme: Theme = {
  id: 'dark',
  name: 'Dark',
  version: '1.0.0',
  author: 'Soul Player Team',
  description: 'Deep midnight theme with violet undertones and elegant contrast',
  isBuiltIn: true,

  colors: {
    // Base: Deep midnight (warm dark with violet hint)
    background: '250 25% 8%',
    foreground: '240 10% 92%',

    // Cards: Elevated night surface
    card: '250 20% 12%',
    'card-foreground': '240 10% 92%',

    // Popovers: Similar to cards
    popover: '250 20% 12%',
    'popover-foreground': '240 10% 92%',

    // Primary: Soft lavender (gentle on dark)
    primary: '260 50% 72%',
    'primary-foreground': '250 25% 8%',

    // Secondary: Muted purple-gray
    secondary: '255 15% 20%',
    'secondary-foreground': '240 10% 92%',

    // Muted: Deep shadow
    muted: '250 18% 15%',
    'muted-foreground': '250 12% 55%',

    // Accent: Warm amber glow (contrast against violet)
    accent: '35 70% 55%',
    'accent-foreground': '250 25% 8%',

    // Destructive: Muted rose
    destructive: '350 55% 48%',
    'destructive-foreground': '240 10% 92%',

    // UI elements
    border: '255 15% 18%',
    input: '255 15% 18%',
    ring: '260 50% 72%',
  },

  gradients: {
    hero: 'linear-gradient(135deg, hsl(250 25% 8%) 0%, hsl(255 20% 14%) 100%)',
    player: 'linear-gradient(to bottom, hsl(250 25% 8%) 0%, hsl(250 18% 12%) 100%)',
    sidebar: 'linear-gradient(to right, hsl(250 18% 14%) 0%, hsl(250 25% 8%) 100%)',
    waveform: 'linear-gradient(90deg, hsl(260 50% 72%) 0%, hsl(35 70% 55%) 50%, hsl(260 50% 72%) 100%)',
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
