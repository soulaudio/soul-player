'use client';

import * as React from 'react';
import { createPortal } from 'react-dom';
import { cn } from '../../lib/utils';

interface DropdownMenuContextValue {
  open: boolean;
  setOpen: (open: boolean) => void;
  triggerRef: React.RefObject<HTMLDivElement | null>;
}

const DropdownMenuContext = React.createContext<DropdownMenuContextValue | null>(null);

export interface DropdownMenuProps {
  children: React.ReactNode;
  onOpenChange?: (open: boolean) => void;
}

export function DropdownMenu({ children, onOpenChange }: DropdownMenuProps) {
  const [open, setOpen] = React.useState(false);
  const triggerRef = React.useRef<HTMLDivElement>(null);

  const handleSetOpen = React.useCallback((newOpen: boolean) => {
    setOpen(newOpen);
    onOpenChange?.(newOpen);
  }, [onOpenChange]);

  return (
    <DropdownMenuContext.Provider value={{ open, setOpen: handleSetOpen, triggerRef }}>
      <div className="relative inline-block" ref={triggerRef}>
        {children}
      </div>
    </DropdownMenuContext.Provider>
  );
}

export interface DropdownMenuTriggerProps {
  children: React.ReactNode;
  asChild?: boolean;
}

export function DropdownMenuTrigger({ children, asChild }: DropdownMenuTriggerProps) {
  const context = React.useContext(DropdownMenuContext);
  if (!context) throw new Error('DropdownMenuTrigger must be used within DropdownMenu');

  const handleClick = () => {
    context.setOpen(!context.open);
  };

  if (asChild && React.isValidElement(children)) {
    return React.cloneElement(children as React.ReactElement<{ onClick?: () => void }>, {
      onClick: handleClick,
    });
  }

  return (
    <button onClick={handleClick}>
      {children}
    </button>
  );
}

export interface DropdownMenuContentProps {
  children: React.ReactNode;
  align?: 'start' | 'center' | 'end';
  className?: string;
}

export function DropdownMenuContent({ children, align = 'center', className }: DropdownMenuContentProps) {
  const context = React.useContext(DropdownMenuContext);
  const ref = React.useRef<HTMLDivElement>(null);
  const [position, setPosition] = React.useState({ top: 0, left: 0 });

  // Calculate position based on trigger element
  const updatePosition = React.useCallback(() => {
    if (!context?.triggerRef.current) return;

    const triggerRect = context.triggerRef.current.getBoundingClientRect();
    const menuWidth = ref.current?.offsetWidth || 320;
    const menuHeight = ref.current?.offsetHeight || 300;
    const padding = 8;

    // Position above the trigger
    let top = triggerRect.top - menuHeight - padding;
    let left = triggerRect.right - menuWidth; // align end

    if (align === 'start') {
      left = triggerRect.left;
    } else if (align === 'center') {
      left = triggerRect.left + triggerRect.width / 2 - menuWidth / 2;
    }

    // Keep within viewport
    left = Math.max(padding, Math.min(left, window.innerWidth - menuWidth - padding));

    // If not enough space above, position below
    if (top < padding) {
      top = triggerRect.bottom + padding;
    }

    setPosition({ top, left });
  }, [context?.triggerRef, align]);

  // Initial position and window event listeners
  React.useEffect(() => {
    if (!context?.open) return;

    updatePosition();
    window.addEventListener('resize', updatePosition);
    window.addEventListener('scroll', updatePosition, true);

    return () => {
      window.removeEventListener('resize', updatePosition);
      window.removeEventListener('scroll', updatePosition, true);
    };
  }, [context?.open, updatePosition]);

  // Watch for content size changes (e.g., when async content loads)
  React.useEffect(() => {
    if (!context?.open || !ref.current) return;

    const resizeObserver = new ResizeObserver(() => {
      updatePosition();
    });

    resizeObserver.observe(ref.current);

    return () => {
      resizeObserver.disconnect();
    };
  }, [context?.open, updatePosition]);

  React.useEffect(() => {
    if (!context?.open) return;

    const handleClickOutside = (event: MouseEvent) => {
      if (ref.current && !ref.current.contains(event.target as Node) &&
          context.triggerRef.current && !context.triggerRef.current.contains(event.target as Node)) {
        context.setOpen(false);
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [context?.open, context]);

  if (!context?.open) return null;

  const content = (
    <div
      ref={ref}
      className={cn(
        'fixed z-[100] min-w-[8rem] overflow-hidden rounded-md border bg-popover p-1 text-popover-foreground shadow-md',
        'animate-in fade-in-0 zoom-in-95',
        className
      )}
      style={{ top: position.top, left: position.left }}
    >
      {children}
    </div>
  );

  // Use portal to render outside of any overflow containers
  if (typeof document !== 'undefined') {
    return createPortal(content, document.body);
  }

  return content;
}

export interface DropdownMenuItemProps {
  children: React.ReactNode;
  onClick?: () => void;
  disabled?: boolean;
  className?: string;
}

export function DropdownMenuItem({ children, onClick, disabled, className }: DropdownMenuItemProps) {
  const context = React.useContext(DropdownMenuContext);

  const handleClick = () => {
    if (disabled) return;
    onClick?.();
    context?.setOpen(false);
  };

  return (
    <div
      className={cn(
        'relative flex cursor-pointer select-none items-center rounded-sm px-2 py-1.5 text-sm outline-none',
        'hover:bg-accent hover:text-accent-foreground',
        'focus:bg-accent focus:text-accent-foreground',
        disabled && 'pointer-events-none opacity-50',
        className
      )}
      onClick={handleClick}
    >
      {children}
    </div>
  );
}

export interface DropdownMenuLabelProps {
  children: React.ReactNode;
  className?: string;
}

export function DropdownMenuLabel({ children, className }: DropdownMenuLabelProps) {
  return (
    <div className={cn('px-2 py-1.5 text-sm font-semibold', className)}>
      {children}
    </div>
  );
}

export function DropdownMenuSeparator() {
  return <div className="-mx-1 my-1 h-px bg-muted" />;
}
