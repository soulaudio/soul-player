import { ReactNode } from 'react'

interface BadgeProps {
  children: ReactNode
  variant?: 'default' | 'outline'
  className?: string
}

export function Badge({ children, variant = 'default', className = '' }: BadgeProps) {
  const baseStyles = 'inline-flex items-center gap-2 px-3 py-1 rounded-full text-xs font-medium transition-all'

  const variants = {
    default: 'bg-violet-500/10 text-violet-300 border border-violet-500/20 backdrop-blur-sm',
    outline: 'border border-zinc-700 text-zinc-400 hover:border-zinc-600'
  }

  return (
    <div className={`${baseStyles} ${variants[variant]} ${className}`}>
      {children}
    </div>
  )
}
