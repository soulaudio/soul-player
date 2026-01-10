/**
 * Built-in themes for Soul Player
 */

import { lightTheme } from './light';
import { darkTheme } from './dark';
import { oceanTheme } from './ocean';
import { earthTheme } from './earth';
import type { Theme } from '../types';

/**
 * Array of all built-in themes
 */
export const builtInThemes: Theme[] = [lightTheme, darkTheme, oceanTheme, earthTheme];

/**
 * Default theme (light)
 */
export const defaultTheme = lightTheme;

/**
 * Get a built-in theme by ID
 */
export function getBuiltInTheme(id: string): Theme | undefined {
  return builtInThemes.find((theme) => theme.id === id);
}

export { lightTheme, darkTheme, oceanTheme, earthTheme };
