import type { Theme } from '../types';

/**
 * Default dark theme for Soul Player
 * Based on the shadcn/ui default dark palette
 */
export const darkTheme: Theme = {
  id: 'dark',
  name: 'Dark',
  version: '1.0.0',
  author: 'Soul Player Team',
  description: 'Sleek dark theme for low-light environments',
  isBuiltIn: true,

  colors: {
    background: '222.2 84% 4.9%',
    foreground: '210 40% 98%',
    card: '222.2 84% 4.9%',
    'card-foreground': '210 40% 98%',
    popover: '222.2 84% 4.9%',
    'popover-foreground': '210 40% 98%',
    primary: '210 40% 98%',
    'primary-foreground': '222.2 47.4% 11.2%',
    secondary: '217.2 32.6% 17.5%',
    'secondary-foreground': '210 40% 98%',
    muted: '217.2 32.6% 17.5%',
    'muted-foreground': '215 20.2% 65.1%',
    accent: '217.2 32.6% 17.5%',
    'accent-foreground': '210 40% 98%',
    destructive: '0 62.8% 30.6%',
    'destructive-foreground': '210 40% 98%',
    border: '217.2 32.6% 17.5%',
    input: '217.2 32.6% 17.5%',
    ring: '212.7 26.8% 83.9%',
  },

  gradients: {
    hero: 'linear-gradient(135deg, hsl(222.2 84% 4.9%) 0%, hsl(217.2 32.6% 17.5%) 100%)',
    player: 'linear-gradient(to bottom, hsl(222.2 84% 4.9%) 0%, hsl(217.2 32.6% 17.5%) 100%)',
    sidebar: 'linear-gradient(to right, hsl(217.2 32.6% 17.5%) 0%, hsl(222.2 84% 4.9%) 100%)',
    waveform: 'linear-gradient(90deg, hsl(210 40% 98%) 0%, hsl(217.2 32.6% 17.5%) 100%)',
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
