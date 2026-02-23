import { memo } from 'react';
import { useTranslation } from 'react-i18next';
import type { TFunction } from 'i18next';

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
  const { t } = useTranslation();
  const info = getQualityInfo(format, bitrate, sampleRate);

  return (
    <span
      className={`
        inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-xs font-medium
        ${info.colorClass}
        ${className}
      `}
      title={getTooltip(t, format, bitrate, sampleRate, channels)}
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
  t: TFunction,
  format: string,
  bitrate?: number,
  sampleRate?: number,
  channels?: number
): string {
  const parts: string[] = [];

  parts.push(t('qualityBadge.tooltipFormat', { format: format.toUpperCase() }));

  if (bitrate) {
    parts.push(t('qualityBadge.tooltipBitrate', { bitrate }));
  }

  if (sampleRate) {
    const kHz = (sampleRate / 1000).toFixed(1);
    parts.push(t('qualityBadge.tooltipSampleRate', { kHz }));
  }

  if (channels) {
    const channelLabel =
      channels === 1 ? t('qualityBadge.tooltipChannelMono') :
      channels === 2 ? t('qualityBadge.tooltipChannelStereo') :
      channels === 6 ? t('qualityBadge.tooltipChannel51') :
      channels === 8 ? t('qualityBadge.tooltipChannel71') :
      t('qualityBadge.tooltipChannelN', { count: channels });
    parts.push(channelLabel);
  }

  // Add quality tier
  const isLossless = ['flac', 'alac', 'wav', 'aiff', 'ape', 'wv'].includes(format.toLowerCase());
  const isHiRes = sampleRate && sampleRate >= 88200;

  if (isHiRes) {
    parts.push(t('qualityBadge.tooltipHiRes'));
  } else if (isLossless) {
    parts.push(t('qualityBadge.tooltipLossless'));
  }

  return parts.join('\n');
}

export default TrackQualityBadge;
