/**
 * Comprehensive tests for SourceIndicator component
 * Tests source type icons, online status, labels, sizes, and tooltips
 */

import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { SourceIndicator } from '../SourceIndicator';

describe('SourceIndicator Component', () => {
  describe('source type icons', () => {
    it('should render folder icon for local source', () => {
      render(<SourceIndicator sourceType="local" />);

      // The component wraps the icon, check for the span container
      const container = screen.getByTitle('Local');
      expect(container).toBeInTheDocument();
    });

    it('should render cloud icon for server source', () => {
      render(<SourceIndicator sourceType="server" />);

      const container = screen.getByTitle(/Server/);
      expect(container).toBeInTheDocument();
    });

    it('should render hard drive icon for cached source', () => {
      render(<SourceIndicator sourceType="cached" />);

      const container = screen.getByTitle('Cached');
      expect(container).toBeInTheDocument();
    });

    it('should default to folder icon for unknown source type', () => {
      // TypeScript will complain but testing runtime behavior
      render(<SourceIndicator sourceType={'unknown' as any} />);

      // Should fall back to local folder icon behavior
      const container = screen.getByTitle('Unknown');
      expect(container).toBeInTheDocument();
    });
  });

  describe('color styling', () => {
    it('should apply muted color for local source', () => {
      render(<SourceIndicator sourceType="local" />);

      const container = screen.getByTitle('Local');
      expect(container.className).toContain('text-muted-foreground');
    });

    it('should apply blue color for online server source', () => {
      render(<SourceIndicator sourceType="server" isOnline={true} />);

      const container = screen.getByTitle(/Server/);
      expect(container.className).toContain('text-blue-400');
    });

    it('should apply muted color for offline server source', () => {
      render(<SourceIndicator sourceType="server" isOnline={false} />);

      const container = screen.getByTitle(/Server/);
      expect(container.className).toContain('text-muted-foreground');
    });

    it('should apply green color for cached source', () => {
      render(<SourceIndicator sourceType="cached" />);

      const container = screen.getByTitle('Cached');
      expect(container.className).toContain('text-green-400');
    });
  });

  describe('online status indicator', () => {
    it('should show wifi icon for online server', () => {
      const { container } = render(<SourceIndicator sourceType="server" isOnline={true} />);

      // Check that there are two icons (cloud + wifi)
      const svgs = container.querySelectorAll('svg');
      expect(svgs.length).toBe(2);
    });

    it('should show wifi-off icon for offline server', () => {
      const { container } = render(<SourceIndicator sourceType="server" isOnline={false} />);

      const svgs = container.querySelectorAll('svg');
      expect(svgs.length).toBe(2);
    });

    it('should apply green color to wifi icon when online', () => {
      const { container } = render(<SourceIndicator sourceType="server" isOnline={true} />);

      const statusSpan = container.querySelector('span > span');
      expect(statusSpan?.className).toContain('text-green-400');
    });

    it('should apply red color to wifi icon when offline', () => {
      const { container } = render(<SourceIndicator sourceType="server" isOnline={false} />);

      const statusSpan = container.querySelector('span > span');
      expect(statusSpan?.className).toContain('text-red-400');
    });

    it('should not show wifi indicator for local source', () => {
      const { container } = render(<SourceIndicator sourceType="local" />);

      const svgs = container.querySelectorAll('svg');
      expect(svgs.length).toBe(1); // Only folder icon
    });

    it('should not show wifi indicator for cached source', () => {
      const { container } = render(<SourceIndicator sourceType="cached" />);

      const svgs = container.querySelectorAll('svg');
      expect(svgs.length).toBe(1); // Only hard drive icon
    });

    it('should default to online status if not specified', () => {
      const { container } = render(<SourceIndicator sourceType="server" />);

      // Should show online wifi icon (green)
      const statusSpan = container.querySelector('span > span');
      expect(statusSpan?.className).toContain('text-green-400');
    });
  });

  describe('labels', () => {
    it('should not show label by default', () => {
      render(<SourceIndicator sourceType="local" />);

      expect(screen.queryByText('Local')).not.toBeInTheDocument();
    });

    it('should show label when showLabel is true', () => {
      render(<SourceIndicator sourceType="local" showLabel={true} />);

      expect(screen.getByText('Local')).toBeInTheDocument();
    });

    it('should show "Server" label for server source', () => {
      render(<SourceIndicator sourceType="server" showLabel={true} />);

      expect(screen.getByText('Server')).toBeInTheDocument();
    });

    it('should show "Cached" label for cached source', () => {
      render(<SourceIndicator sourceType="cached" showLabel={true} />);

      expect(screen.getByText('Cached')).toBeInTheDocument();
    });

    it('should use custom sourceName as label when provided', () => {
      render(
        <SourceIndicator
          sourceType="server"
          sourceName="My Music Server"
          showLabel={true}
        />
      );

      expect(screen.getByText('My Music Server')).toBeInTheDocument();
    });

    it('should use custom sourceName in tooltip', () => {
      render(
        <SourceIndicator
          sourceType="server"
          sourceName="Home NAS"
        />
      );

      const container = screen.getByTitle(/Home NAS/);
      expect(container).toBeInTheDocument();
    });
  });

  describe('tooltips', () => {
    it('should show "Local" tooltip for local source', () => {
      render(<SourceIndicator sourceType="local" />);

      expect(screen.getByTitle('Local')).toBeInTheDocument();
    });

    it('should include online status in server tooltip', () => {
      render(<SourceIndicator sourceType="server" isOnline={true} />);

      const container = screen.getByTitle(/Online/);
      expect(container).toBeInTheDocument();
    });

    it('should include offline status in server tooltip', () => {
      render(<SourceIndicator sourceType="server" isOnline={false} />);

      const container = screen.getByTitle(/Offline/);
      expect(container).toBeInTheDocument();
    });

    it('should show "Cached" tooltip for cached source', () => {
      render(<SourceIndicator sourceType="cached" />);

      expect(screen.getByTitle('Cached')).toBeInTheDocument();
    });

    it('should combine sourceName with online status in tooltip', () => {
      render(
        <SourceIndicator
          sourceType="server"
          sourceName="Cloud Server"
          isOnline={true}
        />
      );

      expect(screen.getByTitle('Cloud Server (Online)')).toBeInTheDocument();
    });

    it('should combine sourceName with offline status in tooltip', () => {
      render(
        <SourceIndicator
          sourceType="server"
          sourceName="NAS Drive"
          isOnline={false}
        />
      );

      expect(screen.getByTitle('NAS Drive (Offline)')).toBeInTheDocument();
    });
  });

  describe('size variants', () => {
    it('should apply small size classes by default', () => {
      const { container } = render(<SourceIndicator sourceType="local" />);

      const svg = container.querySelector('svg');
      expect(svg?.className).toContain('w-3.5');
      expect(svg?.className).toContain('h-3.5');
    });

    it('should apply small size classes when size="sm"', () => {
      const { container } = render(<SourceIndicator sourceType="local" size="sm" />);

      const svg = container.querySelector('svg');
      expect(svg?.className).toContain('w-3.5');
      expect(svg?.className).toContain('h-3.5');
    });

    it('should apply medium size classes when size="md"', () => {
      const { container } = render(<SourceIndicator sourceType="local" size="md" />);

      const svg = container.querySelector('svg');
      expect(svg?.className).toContain('w-4');
      expect(svg?.className).toContain('h-4');
    });

    it('should apply large size classes when size="lg"', () => {
      const { container } = render(<SourceIndicator sourceType="local" size="lg" />);

      const svg = container.querySelector('svg');
      expect(svg?.className).toContain('w-5');
      expect(svg?.className).toContain('h-5');
    });

    it('should maintain correct size for server wifi indicator', () => {
      const { container } = render(
        <SourceIndicator sourceType="server" isOnline={true} size="lg" />
      );

      // Main icon should be large
      const mainIcon = container.querySelector('span > svg');
      expect(mainIcon?.className).toContain('w-5');

      // Wifi indicator should remain small
      const statusSvg = container.querySelector('span > span > svg');
      expect(statusSvg?.className).toContain('w-2.5');
    });
  });

  describe('custom className', () => {
    it('should apply custom className', () => {
      render(<SourceIndicator sourceType="local" className="my-custom-class" />);

      const container = screen.getByTitle('Local');
      expect(container.className).toContain('my-custom-class');
    });

    it('should merge custom className with default classes', () => {
      render(<SourceIndicator sourceType="local" className="additional-style" />);

      const container = screen.getByTitle('Local');
      expect(container.className).toContain('additional-style');
      expect(container.className).toContain('inline-flex');
      expect(container.className).toContain('items-center');
    });
  });

  describe('accessibility', () => {
    it('should have title attribute for screen readers', () => {
      render(<SourceIndicator sourceType="local" />);

      const element = screen.getByTitle('Local');
      expect(element.getAttribute('title')).toBe('Local');
    });

    it('should have descriptive title for server with online status', () => {
      render(
        <SourceIndicator
          sourceType="server"
          sourceName="Music Server"
          isOnline={true}
        />
      );

      expect(screen.getByTitle('Music Server (Online)')).toBeInTheDocument();
    });

    it('should be visually distinct for different source types', () => {
      const { rerender, container } = render(<SourceIndicator sourceType="local" />);
      const localClass = container.firstChild?.className;

      rerender(<SourceIndicator sourceType="server" isOnline={true} />);
      const serverOnlineClass = container.firstChild?.className;

      rerender(<SourceIndicator sourceType="server" isOnline={false} />);
      const serverOfflineClass = container.firstChild?.className;

      rerender(<SourceIndicator sourceType="cached" />);
      const cachedClass = container.firstChild?.className;

      // All should be different
      expect(localClass).not.toBe(serverOnlineClass);
      expect(serverOnlineClass).not.toBe(serverOfflineClass);
      expect(cachedClass).not.toBe(localClass);
    });
  });

  describe('edge cases', () => {
    it('should handle empty sourceName', () => {
      render(
        <SourceIndicator
          sourceType="local"
          sourceName=""
          showLabel={true}
        />
      );

      // Should fall back to default label
      expect(screen.getByText('Local')).toBeInTheDocument();
    });

    it('should handle very long sourceName', () => {
      const longName = 'This is a very long source name that might cause layout issues';
      render(
        <SourceIndicator
          sourceType="server"
          sourceName={longName}
          showLabel={true}
        />
      );

      expect(screen.getByText(longName)).toBeInTheDocument();
    });

    it('should handle special characters in sourceName', () => {
      const specialName = 'Server <>&"\'';
      render(
        <SourceIndicator
          sourceType="server"
          sourceName={specialName}
          showLabel={true}
        />
      );

      expect(screen.getByText(specialName)).toBeInTheDocument();
    });

    it('should handle multiple rapid rerenders', () => {
      const { rerender } = render(<SourceIndicator sourceType="local" />);

      for (let i = 0; i < 20; i++) {
        rerender(<SourceIndicator sourceType="server" isOnline={i % 2 === 0} />);
        rerender(<SourceIndicator sourceType="local" />);
        rerender(<SourceIndicator sourceType="cached" />);
      }

      // Final state should be cached
      expect(screen.getByTitle('Cached')).toBeInTheDocument();
    });
  });

  describe('component composition', () => {
    it('should render correctly with all props', () => {
      render(
        <SourceIndicator
          sourceType="server"
          sourceName="Full Featured Server"
          isOnline={true}
          showLabel={true}
          size="lg"
          className="full-featured"
        />
      );

      const container = screen.getByTitle('Full Featured Server (Online)');
      expect(container).toBeInTheDocument();
      expect(container.className).toContain('full-featured');
      expect(screen.getByText('Full Featured Server')).toBeInTheDocument();
    });

    it('should render multiple instances independently', () => {
      render(
        <>
          <SourceIndicator sourceType="local" sourceName="Local" showLabel={true} />
          <SourceIndicator sourceType="server" sourceName="Server1" isOnline={true} showLabel={true} />
          <SourceIndicator sourceType="server" sourceName="Server2" isOnline={false} showLabel={true} />
          <SourceIndicator sourceType="cached" sourceName="Cached" showLabel={true} />
        </>
      );

      expect(screen.getByText('Local')).toBeInTheDocument();
      expect(screen.getByText('Server1')).toBeInTheDocument();
      expect(screen.getByText('Server2')).toBeInTheDocument();
      expect(screen.getByText('Cached')).toBeInTheDocument();
    });
  });

  describe('rendering performance', () => {
    it('should render efficiently', () => {
      const start = performance.now();

      for (let i = 0; i < 100; i++) {
        const { unmount } = render(
          <SourceIndicator
            sourceType="server"
            sourceName={`Server ${i}`}
            isOnline={true}
            showLabel={true}
          />
        );
        unmount();
      }

      const duration = performance.now() - start;
      expect(duration).toBeLessThan(1000);
    });
  });
});
