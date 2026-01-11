import { memo } from 'react';
import { Folder, Cloud, HardDrive, Wifi, WifiOff } from 'lucide-react';

type SourceType = 'local' | 'server' | 'cached';

interface SourceIndicatorProps {
  sourceType: SourceType;
  sourceName?: string;
  isOnline?: boolean;
  showLabel?: boolean;
  size?: 'sm' | 'md' | 'lg';
  className?: string;
}

/**
 * Displays an icon indicating the source of a track.
 *
 * - Local: folder icon (files on this device)
 * - Server: cloud icon with optional online/offline status
 * - Cached: hard drive icon (downloaded from server)
 */
export const SourceIndicator = memo(function SourceIndicator({
  sourceType,
  sourceName,
  isOnline = true,
  showLabel = false,
  size = 'sm',
  className = '',
}: SourceIndicatorProps) {
  const sizeClasses = {
    sm: 'w-3.5 h-3.5',
    md: 'w-4 h-4',
    lg: 'w-5 h-5',
  };

  const iconSize = sizeClasses[size];

  const getIcon = () => {
    switch (sourceType) {
      case 'local':
        return <Folder className={iconSize} />;
      case 'server':
        return <Cloud className={iconSize} />;
      case 'cached':
        return <HardDrive className={iconSize} />;
      default:
        return <Folder className={iconSize} />;
    }
  };

  const getColorClass = () => {
    switch (sourceType) {
      case 'local':
        return 'text-muted-foreground';
      case 'server':
        return isOnline ? 'text-blue-400' : 'text-muted-foreground';
      case 'cached':
        return 'text-green-400';
      default:
        return 'text-muted-foreground';
    }
  };

  const getTooltip = () => {
    const base = sourceName || getSourceLabel(sourceType);
    if (sourceType === 'server') {
      return `${base} (${isOnline ? 'Online' : 'Offline'})`;
    }
    return base;
  };

  return (
    <span
      className={`inline-flex items-center gap-1.5 ${getColorClass()} ${className}`}
      title={getTooltip()}
    >
      {getIcon()}
      {showLabel && (
        <span className="text-xs">
          {sourceName || getSourceLabel(sourceType)}
        </span>
      )}
      {sourceType === 'server' && (
        <span className={`${sizeClasses.sm} ${isOnline ? 'text-green-400' : 'text-red-400'}`}>
          {isOnline ? <Wifi className="w-2.5 h-2.5" /> : <WifiOff className="w-2.5 h-2.5" />}
        </span>
      )}
    </span>
  );
});

function getSourceLabel(sourceType: SourceType): string {
  switch (sourceType) {
    case 'local':
      return 'Local';
    case 'server':
      return 'Server';
    case 'cached':
      return 'Cached';
    default:
      return 'Unknown';
  }
}

export default SourceIndicator;
