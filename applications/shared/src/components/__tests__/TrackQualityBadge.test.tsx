/**
 * Comprehensive tests for TrackQualityBadge component
 * Tests format detection, quality classification, color coding, and tooltips
 */

import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { TrackQualityBadge } from '../TrackQualityBadge';

describe('TrackQualityBadge Component', () => {
  describe('lossless format detection', () => {
    it('should display FLAC format correctly', () => {
      render(<TrackQualityBadge format="flac" />);

      expect(screen.getByText('FLAC')).toBeInTheDocument();
    });

    it('should display ALAC format correctly', () => {
      render(<TrackQualityBadge format="alac" />);

      expect(screen.getByText('ALAC')).toBeInTheDocument();
    });

    it('should display WAV format correctly', () => {
      render(<TrackQualityBadge format="wav" />);

      expect(screen.getByText('WAV')).toBeInTheDocument();
    });

    it('should display AIFF format correctly', () => {
      render(<TrackQualityBadge format="aiff" />);

      expect(screen.getByText('AIFF')).toBeInTheDocument();
    });

    it('should display APE format correctly', () => {
      render(<TrackQualityBadge format="ape" />);

      expect(screen.getByText('APE')).toBeInTheDocument();
    });

    it('should display WavPack (WV) format correctly', () => {
      render(<TrackQualityBadge format="wv" />);

      expect(screen.getByText('WV')).toBeInTheDocument();
    });

    it('should apply lossless styling to FLAC', () => {
      render(<TrackQualityBadge format="flac" />);

      const badge = screen.getByText('FLAC');
      expect(badge.className).toContain('bg-blue-500/20');
      expect(badge.className).toContain('text-blue-400');
    });

    it('should apply lossless styling to all lossless formats', () => {
      const losslessFormats = ['flac', 'alac', 'wav', 'aiff', 'ape', 'wv'];

      losslessFormats.forEach((format) => {
        const { unmount } = render(<TrackQualityBadge format={format} />);
        const badge = screen.getByText(format.toUpperCase());
        expect(badge.className).toContain('bg-blue-500/20');
        expect(badge.className).toContain('text-blue-400');
        unmount();
      });
    });
  });

  describe('lossy format detection', () => {
    it('should display MP3 format with bitrate', () => {
      render(<TrackQualityBadge format="mp3" bitrate={320} />);

      expect(screen.getByText('MP3 320')).toBeInTheDocument();
    });

    it('should display AAC format with bitrate', () => {
      render(<TrackQualityBadge format="aac" bitrate={256} />);

      expect(screen.getByText('AAC 256')).toBeInTheDocument();
    });

    it('should display OGG format with bitrate', () => {
      render(<TrackQualityBadge format="ogg" bitrate={192} />);

      expect(screen.getByText('OGG 192')).toBeInTheDocument();
    });

    it('should display OPUS format with bitrate', () => {
      render(<TrackQualityBadge format="opus" bitrate={128} />);

      expect(screen.getByText('OPUS 128')).toBeInTheDocument();
    });

    it('should display M4A format with bitrate', () => {
      render(<TrackQualityBadge format="m4a" bitrate={256} />);

      expect(screen.getByText('M4A 256')).toBeInTheDocument();
    });

    it('should display WMA format with bitrate', () => {
      render(<TrackQualityBadge format="wma" bitrate={192} />);

      expect(screen.getByText('WMA 192')).toBeInTheDocument();
    });

    it('should display format only when no bitrate provided', () => {
      render(<TrackQualityBadge format="mp3" />);

      expect(screen.getByText('MP3')).toBeInTheDocument();
    });
  });

  describe('bitrate quality coloring', () => {
    it('should apply green styling for high bitrate (>=256)', () => {
      render(<TrackQualityBadge format="mp3" bitrate={320} />);

      const badge = screen.getByText('MP3 320');
      expect(badge.className).toContain('bg-green-500/20');
      expect(badge.className).toContain('text-green-400');
    });

    it('should apply green styling for exactly 256 kbps', () => {
      render(<TrackQualityBadge format="mp3" bitrate={256} />);

      const badge = screen.getByText('MP3 256');
      expect(badge.className).toContain('bg-green-500/20');
      expect(badge.className).toContain('text-green-400');
    });

    it('should apply yellow styling for lower bitrate (<256)', () => {
      render(<TrackQualityBadge format="mp3" bitrate={192} />);

      const badge = screen.getByText('MP3 192');
      expect(badge.className).toContain('bg-yellow-500/20');
      expect(badge.className).toContain('text-yellow-400');
    });

    it('should apply yellow styling for low bitrate (128)', () => {
      render(<TrackQualityBadge format="mp3" bitrate={128} />);

      const badge = screen.getByText('MP3 128');
      expect(badge.className).toContain('bg-yellow-500/20');
      expect(badge.className).toContain('text-yellow-400');
    });

    it('should apply muted styling when no bitrate for lossy format', () => {
      render(<TrackQualityBadge format="mp3" />);

      const badge = screen.getByText('MP3');
      expect(badge.className).toContain('bg-muted');
      expect(badge.className).toContain('text-muted-foreground');
    });
  });

  describe('hi-res audio detection', () => {
    it('should display Hi-Res badge for 88.2kHz sample rate', () => {
      render(<TrackQualityBadge format="flac" sampleRate={88200} />);

      expect(screen.getByText('Hi-Res 88kHz')).toBeInTheDocument();
    });

    it('should display Hi-Res badge for 96kHz sample rate', () => {
      render(<TrackQualityBadge format="flac" sampleRate={96000} />);

      expect(screen.getByText('Hi-Res 96kHz')).toBeInTheDocument();
    });

    it('should display Hi-Res badge for 176.4kHz sample rate', () => {
      render(<TrackQualityBadge format="flac" sampleRate={176400} />);

      expect(screen.getByText('Hi-Res 176kHz')).toBeInTheDocument();
    });

    it('should display Hi-Res badge for 192kHz sample rate', () => {
      render(<TrackQualityBadge format="flac" sampleRate={192000} />);

      expect(screen.getByText('Hi-Res 192kHz')).toBeInTheDocument();
    });

    it('should apply purple styling for Hi-Res audio', () => {
      render(<TrackQualityBadge format="flac" sampleRate={96000} />);

      const badge = screen.getByText('Hi-Res 96kHz');
      expect(badge.className).toContain('bg-purple-500/20');
      expect(badge.className).toContain('text-purple-400');
    });

    it('should not show Hi-Res for 44.1kHz (CD quality)', () => {
      render(<TrackQualityBadge format="flac" sampleRate={44100} />);

      // Should show just FLAC, not Hi-Res
      expect(screen.getByText('FLAC')).toBeInTheDocument();
      expect(screen.queryByText(/Hi-Res/)).not.toBeInTheDocument();
    });

    it('should not show Hi-Res for 48kHz', () => {
      render(<TrackQualityBadge format="flac" sampleRate={48000} />);

      expect(screen.getByText('FLAC')).toBeInTheDocument();
      expect(screen.queryByText(/Hi-Res/)).not.toBeInTheDocument();
    });

    it('should prioritize Hi-Res over lossless for high sample rate', () => {
      render(<TrackQualityBadge format="flac" sampleRate={192000} />);

      // Should show Hi-Res, not FLAC
      expect(screen.getByText('Hi-Res 192kHz')).toBeInTheDocument();
      expect(screen.queryByText(/^FLAC$/)).not.toBeInTheDocument();
    });
  });

  describe('DSD format detection', () => {
    it('should display DSD format correctly', () => {
      render(<TrackQualityBadge format="dsd64" />);

      expect(screen.getByText('DSD64')).toBeInTheDocument();
    });

    it('should display DSD256 format correctly', () => {
      render(<TrackQualityBadge format="dsd256" />);

      expect(screen.getByText('DSD256')).toBeInTheDocument();
    });

    it('should display DSF format correctly', () => {
      render(<TrackQualityBadge format="dsf" />);

      expect(screen.getByText('DSF')).toBeInTheDocument();
    });

    it('should display DFF format correctly', () => {
      render(<TrackQualityBadge format="dff" />);

      expect(screen.getByText('DFF')).toBeInTheDocument();
    });

    it('should apply purple styling for DSD formats', () => {
      render(<TrackQualityBadge format="dsd256" />);

      const badge = screen.getByText('DSD256');
      expect(badge.className).toContain('bg-purple-500/20');
      expect(badge.className).toContain('text-purple-400');
    });
  });

  describe('unknown format handling', () => {
    it('should display unknown format in uppercase', () => {
      render(<TrackQualityBadge format="xyz" />);

      expect(screen.getByText('XYZ')).toBeInTheDocument();
    });

    it('should apply muted styling for unknown formats', () => {
      render(<TrackQualityBadge format="unknown" />);

      const badge = screen.getByText('UNKNOWN');
      expect(badge.className).toContain('bg-muted');
      expect(badge.className).toContain('text-muted-foreground');
    });

    it('should display "Unknown" for empty format', () => {
      render(<TrackQualityBadge format="" />);

      expect(screen.getByText('Unknown')).toBeInTheDocument();
    });
  });

  describe('tooltip content', () => {
    it('should include format in tooltip', () => {
      render(<TrackQualityBadge format="flac" />);

      const badge = screen.getByText('FLAC');
      expect(badge.getAttribute('title')).toContain('Format: FLAC');
    });

    it('should include bitrate in tooltip when provided', () => {
      render(<TrackQualityBadge format="mp3" bitrate={320} />);

      const badge = screen.getByText('MP3 320');
      expect(badge.getAttribute('title')).toContain('Bitrate: 320 kbps');
    });

    it('should include sample rate in tooltip when provided', () => {
      render(<TrackQualityBadge format="flac" sampleRate={96000} />);

      const badge = screen.getByText('Hi-Res 96kHz');
      expect(badge.getAttribute('title')).toContain('Sample Rate: 96.0 kHz');
    });

    it('should include stereo channel info in tooltip', () => {
      render(<TrackQualityBadge format="flac" channels={2} />);

      const badge = screen.getByText('FLAC');
      expect(badge.getAttribute('title')).toContain('Stereo');
    });

    it('should include mono channel info in tooltip', () => {
      render(<TrackQualityBadge format="mp3" bitrate={128} channels={1} />);

      const badge = screen.getByText('MP3 128');
      expect(badge.getAttribute('title')).toContain('Mono');
    });

    it('should include 5.1 surround info in tooltip', () => {
      render(<TrackQualityBadge format="flac" channels={6} />);

      const badge = screen.getByText('FLAC');
      expect(badge.getAttribute('title')).toContain('5.1 Surround');
    });

    it('should include 7.1 surround info in tooltip', () => {
      render(<TrackQualityBadge format="flac" channels={8} />);

      const badge = screen.getByText('FLAC');
      expect(badge.getAttribute('title')).toContain('7.1 Surround');
    });

    it('should include lossless quality tier in tooltip', () => {
      render(<TrackQualityBadge format="flac" />);

      const badge = screen.getByText('FLAC');
      expect(badge.getAttribute('title')).toContain('Lossless Audio');
    });

    it('should include hi-res quality tier in tooltip', () => {
      render(<TrackQualityBadge format="flac" sampleRate={192000} />);

      const badge = screen.getByText('Hi-Res 192kHz');
      expect(badge.getAttribute('title')).toContain('High Resolution Audio');
    });
  });

  describe('case insensitivity', () => {
    it('should handle uppercase format input', () => {
      render(<TrackQualityBadge format="FLAC" />);

      expect(screen.getByText('FLAC')).toBeInTheDocument();
    });

    it('should handle mixed case format input', () => {
      render(<TrackQualityBadge format="FlAc" />);

      expect(screen.getByText('FLAC')).toBeInTheDocument();
    });

    it('should apply correct styling regardless of case', () => {
      render(<TrackQualityBadge format="Mp3" bitrate={320} />);

      const badge = screen.getByText('MP3 320');
      expect(badge.className).toContain('bg-green-500/20');
    });
  });

  describe('custom className', () => {
    it('should apply custom className', () => {
      render(<TrackQualityBadge format="flac" className="custom-class" />);

      const badge = screen.getByText('FLAC');
      expect(badge.className).toContain('custom-class');
    });

    it('should merge custom className with default classes', () => {
      render(<TrackQualityBadge format="flac" className="my-custom-class" />);

      const badge = screen.getByText('FLAC');
      expect(badge.className).toContain('my-custom-class');
      expect(badge.className).toContain('inline-flex');
      expect(badge.className).toContain('bg-blue-500/20');
    });
  });

  describe('edge cases', () => {
    it('should handle very high bitrates', () => {
      render(<TrackQualityBadge format="mp3" bitrate={500} />);

      expect(screen.getByText('MP3 500')).toBeInTheDocument();
    });

    it('should handle very low bitrates', () => {
      render(<TrackQualityBadge format="mp3" bitrate={32} />);

      const badge = screen.getByText('MP3 32');
      expect(badge.className).toContain('bg-yellow-500/20');
    });

    it('should handle unusual sample rates', () => {
      render(<TrackQualityBadge format="flac" sampleRate={352800} />);

      expect(screen.getByText('Hi-Res 353kHz')).toBeInTheDocument();
    });

    it('should handle sample rate of exactly 88200', () => {
      render(<TrackQualityBadge format="flac" sampleRate={88200} />);

      expect(screen.getByText('Hi-Res 88kHz')).toBeInTheDocument();
    });

    it('should handle sample rate just below hi-res threshold', () => {
      render(<TrackQualityBadge format="flac" sampleRate={88199} />);

      expect(screen.getByText('FLAC')).toBeInTheDocument();
    });

    it('should handle unusual channel counts', () => {
      render(<TrackQualityBadge format="flac" channels={4} />);

      const badge = screen.getByText('FLAC');
      expect(badge.getAttribute('title')).toContain('4 channels');
    });

    it('should handle zero bitrate', () => {
      render(<TrackQualityBadge format="mp3" bitrate={0} />);

      // 0 is falsy, so should show format only
      expect(screen.getByText('MP3')).toBeInTheDocument();
    });
  });

  describe('rendering optimization', () => {
    it('should render quickly with minimal props', () => {
      const start = performance.now();

      for (let i = 0; i < 100; i++) {
        const { unmount } = render(<TrackQualityBadge format="flac" />);
        unmount();
      }

      const duration = performance.now() - start;
      expect(duration).toBeLessThan(1000); // Should render 100 times in under 1 second
    });

    it('should render quickly with all props', () => {
      const start = performance.now();

      for (let i = 0; i < 100; i++) {
        const { unmount } = render(
          <TrackQualityBadge
            format="flac"
            bitrate={320}
            sampleRate={96000}
            channels={2}
          />
        );
        unmount();
      }

      const duration = performance.now() - start;
      expect(duration).toBeLessThan(1000);
    });
  });
});
