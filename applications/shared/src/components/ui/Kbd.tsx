import { formatShortcut } from '../../lib/platform';

interface KbdProps {
  /**
   * Array of keys making up the shortcut
   * Use 'mod' for the platform-specific modifier key (Cmd on Mac, Ctrl elsewhere)
   * @example ['mod', 'k'] => "⌘K" on Mac, "Ctrl+K" on Windows/Linux
   */
  keys: string[];
  /**
   * Optional className for custom styling
   */
  className?: string;
  /**
   * Size variant
   */
  size?: 'sm' | 'md';
}

/**
 * Kbd component for displaying keyboard shortcuts
 * Follows common UI patterns used in VS Code, Linear, and other modern apps
 */
export function Kbd({ keys, className = '', size = 'sm' }: KbdProps) {
  const shortcut = formatShortcut(keys);

  const sizeClasses = {
    sm: 'text-[10px] px-1 py-0.5 min-w-[16px]',
    md: 'text-xs px-1.5 py-0.5 min-w-[20px]',
  };

  return (
    <kbd
      className={`
        inline-flex items-center justify-center
        font-mono font-medium
        rounded
        border border-border
        bg-muted/50
        text-muted-foreground
        shadow-sm
        ${sizeClasses[size]}
        ${className}
      `}
    >
      {shortcut}
    </kbd>
  );
}

/**
 * KbdGroup component for displaying multiple key combinations
 * @example <KbdGroup shortcuts={[['mod', 'k'], ['mod', 'shift', 'p']]} />
 */
export function KbdGroup({
  shortcuts,
  className = '',
}: {
  shortcuts: string[][];
  className?: string;
}) {
  return (
    <span className={`inline-flex items-center gap-1 ${className}`}>
      {shortcuts.map((keys, index) => (
        <span key={index} className="inline-flex items-center gap-0.5">
          {index > 0 && <span className="text-muted-foreground text-xs mx-0.5">or</span>}
          <Kbd keys={keys} />
        </span>
      ))}
    </span>
  );
}
