import React from 'react'
import { useTranslation } from 'react-i18next'
import { Grid3x3, List } from 'lucide-react'

interface ViewToggleProps {
  view: 'grid' | 'list'
  onViewChange: (view: 'grid' | 'list') => void
  className?: string
}

export function ViewToggle({ view, onViewChange, className = '' }: ViewToggleProps) {
  const { t } = useTranslation()

  return (
    <div className={`flex gap-1 rounded-lg bg-neutral-800 p-1 ${className}`}>
      <button
        onClick={() => onViewChange('grid')}
        className={`flex items-center gap-2 rounded-md px-3 py-1.5 text-sm font-medium transition-colors ${
          view === 'grid'
            ? 'bg-neutral-700 text-white'
            : 'text-neutral-400 hover:text-white hover:bg-neutral-750'
        }`}
        aria-label={t('artist.viewGrid')}
        aria-pressed={view === 'grid'}
      >
        <Grid3x3 size={16} />
        <span className="hidden sm:inline">{t('artist.viewGrid')}</span>
      </button>
      <button
        onClick={() => onViewChange('list')}
        className={`flex items-center gap-2 rounded-md px-3 py-1.5 text-sm font-medium transition-colors ${
          view === 'list'
            ? 'bg-neutral-700 text-white'
            : 'text-neutral-400 hover:text-white hover:bg-neutral-750'
        }`}
        aria-label={t('artist.viewList')}
        aria-pressed={view === 'list'}
      >
        <List size={16} />
        <span className="hidden sm:inline">{t('artist.viewList')}</span>
      </button>
    </div>
  )
}
