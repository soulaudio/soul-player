import type { Theme } from '../types';

/**
 * Earth theme for Soul Player
 * Nature/forest palette with deep greens, moss, bark browns, and soil tones
 * Grounded, organic feel inspired by forests and natural landscapes
 */
export const earthTheme: Theme = {
  id: 'earth',
  name: 'Earth',
  version: '1.1.0',
  author: 'Soul Player Team',
  description: 'Forest-inspired theme with moss greens and earthy browns',
  isBuiltIn: true,

  colors: {
    // Base: Deep forest floor (very dark green-brown)
    background: '120 18% 7%',
    foreground: '90 15% 88%',

    // Cards: Dark bark/moss tone
    card: '110 18% 11%',
    'card-foreground': '90 15% 88%',

    // Popovers: Similar to cards
    popover: '110 18% 11%',
    'popover-foreground': '90 15% 88%',

    // Primary: Moss/sage green (muted forest green)
    primary: '100 25% 42%',
    'primary-foreground': '120 18% 7%',

    // Secondary: Bark brown
    secondary: '30 25% 25%',
    'secondary-foreground': '90 15% 88%',

    // Muted: Deep forest shadow
    muted: '110 15% 14%',
    'muted-foreground': '90 10% 55%',

    // Accent: Rich soil/copper brown
    accent: '25 40% 38%',
    'accent-foreground': '90 15% 92%',

    // Destructive: Autumn red (fallen leaves)
    destructive: '8 50% 42%',
    'destructive-foreground': '90 15% 92%',

    // UI elements
    border: '110 12% 18%',
    input: '110 12% 18%',
    ring: '100 25% 42%',
  },

  gradients: {
    hero: 'linear-gradient(135deg, hsl(120 18% 7%) 0%, hsl(110 18% 14%) 100%)',
    player: 'linear-gradient(to bottom, hsl(120 18% 7%) 0%, hsl(110 15% 10%) 100%)',
    sidebar: 'linear-gradient(to right, hsl(110 15% 12%) 0%, hsl(120 18% 7%) 100%)',
    waveform: 'linear-gradient(90deg, hsl(100 25% 42%) 0%, hsl(25 40% 38%) 50%, hsl(100 25% 42%) 100%)',
  },

  typography: {
    fontFamily: {
      sans: ['Georgia', 'Cambria', 'Times New Roman', 'Times', 'serif'],
      mono: ['JetBrains Mono', 'Consolas', 'monospace'],
    },
    fontSize: {
      base: '16px',
    },
  },
};
