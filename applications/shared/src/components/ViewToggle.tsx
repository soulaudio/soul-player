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
    <div className={`flex gap-1 rounded-lg bg-muted p-1 ${className}`}>
      <button
        onClick={() => onViewChange('grid')}
        data-state={view === 'grid' ? 'active' : 'inactive'}
        aria-pressed={view === 'grid'}
        aria-label={t('artist.viewGrid')}
        className={`flex items-center gap-2 rounded-md px-3 py-1.5 text-sm font-medium transition-all ${
          view === 'grid'
            ? 'bg-primary/10 text-primary'
            : 'text-muted-foreground hover:opacity-80 hover:bg-foreground/10'
        }`}
      >
        <Grid3x3 size={16} />
        <span className="hidden sm:inline">{t('artist.viewGrid')}</span>
      </button>
      <button
        onClick={() => onViewChange('list')}
        data-state={view === 'list' ? 'active' : 'inactive'}
        aria-pressed={view === 'list'}
        aria-label={t('artist.viewList')}
        className={`flex items-center gap-2 rounded-md px-3 py-1.5 text-sm font-medium transition-all ${
          view === 'list'
            ? 'bg-primary/10 text-primary'
            : 'text-muted-foreground hover:opacity-80 hover:bg-foreground/10'
        }`}
      >
        <List size={16} />
        <span className="hidden sm:inline">{t('artist.viewList')}</span>
      </button>
    </div>
  )
}
