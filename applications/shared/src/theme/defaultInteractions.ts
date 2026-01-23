/**
 * Default interaction configuration
 * Following Tailwind v4 best practices with CSS-first approach
 */

import type { ThemeInteractions } from './types';

/**
 * Default interaction values used across all themes
 * Themes can override these values for custom behavior
 */
export const defaultInteractions: ThemeInteractions = {
  hover: {
    textOpacity: 0.8, // 80% opacity for text/icons on hover
    buttonOpacity: 0.9, // 90% opacity for buttons (more subtle)
    bgOpacity: 0.1, // 10% background overlay on hover
  },
  selected: {
    bgOpacity: 0.2, // 20% background for selected/active items
  },
  disabled: {
    opacity: 0.5, // 50% opacity for disabled elements
  },
  transition: {
    duration: '200ms', // Standard transition duration
  },
};
