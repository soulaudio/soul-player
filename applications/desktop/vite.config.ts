import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'path';
import { resolve } from 'path';

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [
    react({
      // Fast Refresh is enabled by default in @vitejs/plugin-react
      // No additional configuration needed!
    }),
  ],

  // Vite options tailored for Tauri
  clearScreen: false,

  // Build configuration for multiple HTML entry points
  build: {
    rollupOptions: {
      input: {
        main: resolve(__dirname, 'index.html'),
        splash: resolve(__dirname, 'splash.html'),
      },
    },
    // Optimize for Tauri's modern webview
    target: 'esnext',
    minify: 'esbuild',
    // Source maps for production debugging (if needed)
    sourcemap: process.env.TAURI_ENV_DEBUG === 'true',
    // Increase chunk size warning limit
    chunkSizeWarningLimit: 1000,
  },

  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    open: false, // Don't auto-open browser (Tauri manages windows)
    hmr: host
      ? {
          protocol: 'ws',
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // Ignore Rust files to prevent unnecessary watchers
      ignored: [
        '**/src-tauri/**',
        '**/target/**',
        '**/.git/**',
        '**/node_modules/**',
        '**/.yarn/**',
        '**/dist/**',
      ],
      // Use native file events (faster than polling)
      usePolling: false,
    },
    // Enable WebSocket compression for faster HMR
    ws: true,
  },

  // Optimize dependencies for faster dev server startup
  optimizeDeps: {
    // Pre-bundle heavy dependencies
    include: [
      'react',
      'react-dom',
      'react-router-dom',
      'zustand',
      'i18next',
      'react-i18next',
      'lucide-react',
      '@tanstack/react-virtual', // Large virtualization library
    ],
    // Exclude Tauri API from pre-bundling (it's dynamic)
    exclude: ['@tauri-apps/api', '@tauri-apps/plugin-shell'],
  },

  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
      '@shared': path.resolve(__dirname, '../shared/src'),
      // Direct alias for workspace package so Vite watches the real source
      // directory instead of the node_modules symlink (fixes HMR for shared code)
      '@soul-player/shared': path.resolve(__dirname, '../shared/src'),
    },
  },
});
