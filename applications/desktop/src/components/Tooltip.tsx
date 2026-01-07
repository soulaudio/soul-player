import { ReactNode, useState, useRef, useEffect } from 'react';
import { Kbd } from './Kbd';

interface TooltipProps {
  /**
   * The element that triggers the tooltip
   */
  children: ReactNode;
  /**
   * Tooltip content (text or component)
   */
  content: ReactNode;
  /**
   * Optional keyboard shortcut to display
   */
  shortcut?: string[];
  /**
   * Tooltip position
   */
  position?: 'top' | 'bottom' | 'left' | 'right';
  /**
   * Delay before showing tooltip (ms)
   */
  delay?: number;
  /**
   * Disable the tooltip
   */
  disabled?: boolean;
}

/**
 * Tooltip component with keyboard shortcut support
 * Follows accessibility best practices (ARIA attributes, keyboard navigation)
 */
export function Tooltip({
  children,
  content,
  shortcut,
  position = 'bottom',
  delay = 500,
  disabled = false,
}: TooltipProps) {
  const [isVisible, setIsVisible] = useState(false);
  const timeoutRef = useRef<number | null>(null);
  const tooltipRef = useRef<HTMLDivElement>(null);

  const showTooltip = () => {
    if (disabled) return;
    timeoutRef.current = window.setTimeout(() => {
      setIsVisible(true);
    }, delay);
  };

  const hideTooltip = () => {
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
      timeoutRef.current = null;
    }
    setIsVisible(false);
  };

  useEffect(() => {
    return () => {
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
      }
    };
  }, []);

  const positionClasses = {
    top: 'bottom-full left-1/2 -translate-x-1/2 mb-2',
    bottom: 'top-full left-1/2 -translate-x-1/2 mt-2',
    left: 'right-full top-1/2 -translate-y-1/2 mr-2',
    right: 'left-full top-1/2 -translate-y-1/2 ml-2',
  };

  return (
    <div
      className="relative inline-flex"
      onMouseEnter={showTooltip}
      onMouseLeave={hideTooltip}
      onFocus={showTooltip}
      onBlur={hideTooltip}
    >
      {children}

      {isVisible && (
        <div
          ref={tooltipRef}
          role="tooltip"
          className={`
            absolute z-50
            px-2 py-1.5
            rounded-md
            bg-popover
            border border-border
            text-popover-foreground
            text-xs
            whitespace-nowrap
            shadow-lg
            pointer-events-none
            animate-in fade-in-0 zoom-in-95
            ${positionClasses[position]}
          `}
        >
          <div className="flex items-center gap-2">
            <span>{content}</span>
            {shortcut && <Kbd keys={shortcut} />}
          </div>
        </div>
      )}
    </div>
  );
}

/**
 * TooltipButton - Convenience component for buttons with tooltips
 */
interface TooltipButtonProps {
  tooltip: string;
  shortcut?: string[];
  onClick?: () => void;
  children: ReactNode;
  className?: string;
  disabled?: boolean;
  ariaLabel?: string;
}

export function TooltipButton({
  tooltip,
  shortcut,
  onClick,
  children,
  className = '',
  disabled = false,
  ariaLabel,
}: TooltipButtonProps) {
  return (
    <Tooltip content={tooltip} shortcut={shortcut} disabled={disabled}>
      <button
        onClick={onClick}
        className={className}
        disabled={disabled}
        aria-label={ariaLabel || tooltip}
      >
        {children}
      </button>
    </Tooltip>
  );
}
