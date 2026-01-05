import type { Theme } from '../types';

/**
 * Default light theme for Soul Player
 * Based on the shadcn/ui default light palette
 */
export const lightTheme: Theme = {
  id: 'light',
  name: 'Light',
  version: '1.0.0',
  author: 'Soul Player Team',
  description: 'Clean and bright default theme',
  isBuiltIn: true,

  colors: {
    background: '0 0% 100%',
    foreground: '222.2 84% 4.9%',
    card: '0 0% 100%',
    'card-foreground': '222.2 84% 4.9%',
    popover: '0 0% 100%',
    'popover-foreground': '222.2 84% 4.9%',
    primary: '222.2 47.4% 11.2%',
    'primary-foreground': '210 40% 98%',
    secondary: '210 40% 96.1%',
    'secondary-foreground': '222.2 47.4% 11.2%',
    muted: '210 40% 96.1%',
    'muted-foreground': '215.4 16.3% 46.9%',
    accent: '210 40% 96.1%',
    'accent-foreground': '222.2 47.4% 11.2%',
    destructive: '0 84.2% 60.2%',
    'destructive-foreground': '210 40% 98%',
    border: '214.3 31.8% 91.4%',
    input: '214.3 31.8% 91.4%',
    ring: '222.2 84% 4.9%',
  },

  gradients: {
    hero: 'linear-gradient(135deg, hsl(210 40% 98%) 0%, hsl(214.3 31.8% 91.4%) 100%)',
    player: 'linear-gradient(to bottom, hsl(0 0% 100%) 0%, hsl(210 40% 96.1%) 100%)',
    sidebar: 'linear-gradient(to right, hsl(210 40% 96.1%) 0%, hsl(0 0% 100%) 100%)',
    waveform: 'linear-gradient(90deg, hsl(222.2 47.4% 11.2%) 0%, hsl(210 40% 96.1%) 100%)',
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
