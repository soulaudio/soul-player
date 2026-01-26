/**
 * Setup verification tests
 * These tests verify that the test infrastructure is properly configured
 */

import { describe, it, expect } from 'vitest';
import { screen } from '@testing-library/react';
import { renderDemoApp, createSampleDemoData } from './test-setup';
import { setupAudioMocks } from './mocks';

describe('E2E Test Setup', () => {
  describe('Test Infrastructure', () => {
    it('should have vitest globals available', () => {
      expect(describe).toBeDefined();
      expect(it).toBeDefined();
      expect(expect).toBeDefined();
    });

    it('should have testing library available', () => {
      expect(screen).toBeDefined();
      expect(renderDemoApp).toBeDefined();
    });

    it('should create sample demo data', () => {
      const data = createSampleDemoData();
      expect(data.tracks).toHaveLength(5);
      expect(data.albums).toHaveLength(2);
      expect(data.playlists).toHaveLength(1);
    });
  });

  describe('Audio Mocking', () => {
    it('should setup audio mocks', () => {
      const cleanup = setupAudioMocks();
      expect(globalThis.Audio).toBeDefined();
      expect(globalThis.HTMLAudioElement).toBeDefined();
      cleanup();
    });

    it('should create mock audio element', () => {
      const cleanup = setupAudioMocks();
      const audio = new Audio();
      expect(audio.src).toBe('');
      expect(audio.paused).toBe(true);
      expect(audio.volume).toBe(1);
      cleanup();
    });

    it('should simulate audio playback', async () => {
      const cleanup = setupAudioMocks();
      const audio = new Audio();
      audio.src = '/test.mp3';

      await audio.play();
      expect(audio.paused).toBe(false);

      audio.pause();
      expect(audio.paused).toBe(true);

      cleanup();
    });
  });

  describe('Demo App Rendering', () => {
    it('should render demo app with loading state', async () => {
      const result = await renderDemoApp();
      const cleanup = (result as any).cleanup;

      // Demo should have loaded successfully
      expect(screen.queryByText(/Loading demo/i)).not.toBeInTheDocument();

      cleanup();
    });

    it('should render demo app with sample data', async () => {
      const data = createSampleDemoData();
      const result = await renderDemoApp(data);
      const cleanup = (result as any).cleanup;

      // Should not show error
      expect(screen.queryByText(/Error/i)).not.toBeInTheDocument();

      cleanup();
    });
  });
});
