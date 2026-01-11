import { memo } from 'react';

interface TrackQualityBadgeProps {
  format: string;
  bitrate?: number;
  sampleRate?: number;
  channels?: number;
  className?: string;
}

/**
 * Displays audio quality information as a compact badge.
 *
 * Examples:
 * - "FLAC" (lossless)
 * - "MP3 320" (lossy with bitrate)
 * - "Hi-Res 96kHz" (high resolution)
 * - "DSD256" (for DSD files)
 */
export const TrackQualityBadge = memo(function TrackQualityBadge({
  format,
  bitrate,
  sampleRate,
  channels,
  className = '',
}: TrackQualityBadgeProps) {
  const info = getQualityInfo(format, bitrate, sampleRate);

  return (
    <span
      className={`
        inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-xs font-medium
        ${info.colorClass}
        ${className}
      `}
      title={getTooltip(format, bitrate, sampleRate, channels)}
    >
      {info.label}
    </span>
  );
});

interface QualityInfo {
  label: string;
  colorClass: string;
}

function getQualityInfo(
  format: string,
  bitrate?: number,
  sampleRate?: number
): QualityInfo {
  const formatUpper = format.toUpperCase();

  // Check for Hi-Res (88.2kHz or higher)
  if (sampleRate && sampleRate >= 88200) {
    const kHz = Math.round(sampleRate / 1000);
    return {
      label: `Hi-Res ${kHz}kHz`,
      colorClass: 'bg-purple-500/20 text-purple-400',
    };
  }

  // Lossless formats
  if (['FLAC', 'ALAC', 'WAV', 'AIFF', 'APE', 'WV'].includes(formatUpper)) {
    return {
      label: formatUpper,
      colorClass: 'bg-blue-500/20 text-blue-400',
    };
  }

  // DSD formats
  if (formatUpper.startsWith('DSD') || formatUpper === 'DSF' || formatUpper === 'DFF') {
    return {
      label: formatUpper,
      colorClass: 'bg-purple-500/20 text-purple-400',
    };
  }

  // Lossy formats with bitrate
  if (['MP3', 'AAC', 'OGG', 'OPUS', 'M4A', 'WMA'].includes(formatUpper)) {
    if (bitrate && bitrate >= 256) {
      return {
        label: `${formatUpper} ${bitrate}`,
        colorClass: 'bg-green-500/20 text-green-400',
      };
    } else if (bitrate) {
      return {
        label: `${formatUpper} ${bitrate}`,
        colorClass: 'bg-yellow-500/20 text-yellow-400',
      };
    } else {
      return {
        label: formatUpper,
        colorClass: 'bg-muted text-muted-foreground',
      };
    }
  }

  // Unknown format
  return {
    label: formatUpper || 'Unknown',
    colorClass: 'bg-muted text-muted-foreground',
  };
}

function getTooltip(
  format: string,
  bitrate?: number,
  sampleRate?: number,
  channels?: number
): string {
  const parts: string[] = [];

  parts.push(`Format: ${format.toUpperCase()}`);

  if (bitrate) {
    parts.push(`Bitrate: ${bitrate} kbps`);
  }

  if (sampleRate) {
    const kHz = (sampleRate / 1000).toFixed(1);
    parts.push(`Sample Rate: ${kHz} kHz`);
  }

  if (channels) {
    const channelLabel =
      channels === 1 ? 'Mono' :
      channels === 2 ? 'Stereo' :
      channels === 6 ? '5.1 Surround' :
      channels === 8 ? '7.1 Surround' :
      `${channels} channels`;
    parts.push(channelLabel);
  }

  // Add quality tier
  const isLossless = ['flac', 'alac', 'wav', 'aiff', 'ape', 'wv'].includes(format.toLowerCase());
  const isHiRes = sampleRate && sampleRate >= 88200;

  if (isHiRes) {
    parts.push('High Resolution Audio');
  } else if (isLossless) {
    parts.push('Lossless Audio');
  }

  return parts.join('\n');
}

export default TrackQualityBadge;
