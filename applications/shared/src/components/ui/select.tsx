'use client';

import * as React from 'react';
import { cn } from '../../lib/utils';
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
} from './dropdown-menu';

export interface SelectOption {
  value: string;
  label: string;
  disabled?: boolean;
}

export interface SelectGroup {
  label?: string;
  options: SelectOption[];
}

export interface SelectProps {
  value: string;
  onChange: (value: string) => void;
  options?: SelectOption[];
  groups?: SelectGroup[];
  placeholder?: string;
  disabled?: boolean;
  className?: string;
  align?: 'start' | 'center' | 'end';
  'data-testid'?: string;
}

export function Select({
  value,
  onChange,
  options,
  groups,
  placeholder = 'Select…',
  disabled,
  className,
  align = 'start',
  'data-testid': dataTestId,
}: SelectProps) {
  // Flatten all options to find the selected label
  const allOptions = groups
    ? groups.flatMap(g => g.options)
    : (options ?? []);
  const selectedLabel = allOptions.find(o => o.value === value)?.label;

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <button
          disabled={disabled}
          data-testid={dataTestId}
          className={cn(
            'flex items-center justify-between gap-2 px-3 py-2 text-sm rounded-lg',
            'border border-border bg-background text-foreground',
            'hover:bg-foreground/[var(--hover-bg-opacity)]',
            'transition-colors duration-[var(--transition-duration)]',
            'focus:outline-none focus:ring-2 focus:ring-primary',
            'disabled:opacity-[var(--disabled-opacity)] disabled:cursor-not-allowed',
            className
          )}
        >
          <span className={cn(!selectedLabel && 'text-muted-foreground')}>
            {selectedLabel ?? placeholder}
          </span>
          <svg
            className="w-4 h-4 text-muted-foreground flex-shrink-0"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            strokeWidth={2}
          >
            <path strokeLinecap="round" strokeLinejoin="round" d="M19 9l-7 7-7-7" />
          </svg>
        </button>
      </DropdownMenuTrigger>

      <DropdownMenuContent align={align}>
        {groups
          ? groups.map((group, i) => (
              <React.Fragment key={i}>
                {i > 0 && <DropdownMenuSeparator />}
                {group.label && <DropdownMenuLabel>{group.label}</DropdownMenuLabel>}
                {group.options.map(option => (
                  <DropdownMenuItem
                    key={option.value}
                    onClick={() => onChange(option.value)}
                    disabled={option.disabled}
                    className={option.value === value ? 'text-primary' : ''}
                  >
                    {option.label}
                  </DropdownMenuItem>
                ))}
              </React.Fragment>
            ))
          : options?.map(option => (
              <DropdownMenuItem
                key={option.value}
                onClick={() => onChange(option.value)}
                disabled={option.disabled}
                className={option.value === value ? 'text-primary' : ''}
              >
                {option.label}
              </DropdownMenuItem>
            ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
