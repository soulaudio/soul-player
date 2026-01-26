/**
 * Debug utility for conditional logging.
 * Logs are only shown in development mode to reduce bundle size and prevent data exposure in production.
 */

/**
 * Check if we're in development mode.
 * Uses process.env.NODE_ENV for compatibility across build tools.
 */
const isDevelopment = process.env.NODE_ENV === 'development';

/**
 * Debug logging utility.
 * All logs are disabled in production builds.
 */
export const debug = {
  /**
   * Log a debug message (only in development).
   */
  log: (...args: unknown[]) => {
    if (isDevelopment) {
      console.log(...args);
    }
  },

  /**
   * Log a warning message (only in development).
   */
  warn: (...args: unknown[]) => {
    if (isDevelopment) {
      console.warn(...args);
    }
  },

  /**
   * Log an error message.
   * Errors are always logged, even in production, as they indicate critical issues.
   */
  error: (...args: unknown[]) => {
    console.error(...args);
  },
};
