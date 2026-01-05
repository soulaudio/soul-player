import type { Theme } from '../types';

/**
 * Ocean theme for Soul Player
 * Cool blue/teal palette inspired by ocean depths
 */
export const oceanTheme: Theme = {
  id: 'ocean',
  name: 'Ocean',
  version: '1.0.0',
  author: 'Soul Player Team',
  description: 'Cool and calming ocean-inspired theme with blue and teal accents',
  isBuiltIn: true,

  colors: {
    background: '210 100% 97%',
    foreground: '210 60% 15%',
    card: '200 50% 98%',
    'card-foreground': '210 60% 15%',
    popover: '200 50% 98%',
    'popover-foreground': '210 60% 15%',
    primary: '200 90% 50%',
    'primary-foreground': '210 100% 98%',
    secondary: '180 85% 55%',
    'secondary-foreground': '210 100% 98%',
    muted: '200 40% 92%',
    'muted-foreground': '200 25% 45%',
    accent: '180 85% 55%',
    'accent-foreground': '210 100% 98%',
    destructive: '0 84.2% 60.2%',
    'destructive-foreground': '210 100% 98%',
    border: '200 30% 85%',
    input: '200 30% 85%',
    ring: '200 90% 50%',
  },

  gradients: {
    hero: 'linear-gradient(135deg, hsl(200 90% 50%) 0%, hsl(180 85% 55%) 100%)',
    player: 'linear-gradient(to bottom, hsl(210 100% 97%) 0%, hsl(200 80% 92%) 100%)',
    sidebar: 'linear-gradient(to right, hsl(200 40% 92%) 0%, hsl(210 100% 97%) 100%)',
    waveform: 'linear-gradient(90deg, hsl(200 90% 50%) 0%, hsl(180 85% 55%) 50%, hsl(200 90% 50%) 100%)',
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
