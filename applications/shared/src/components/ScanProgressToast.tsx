/**
 * Scan Progress Toast - displays scanning progress with debounced cache invalidation
 */

import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Loader2, CheckCircle2, X } from 'lucide-react'
import { useScanProgress, useScanCompletionInvalidation } from '../hooks/useScanCompletionInvalidation'

export function ScanProgressToast() {
  const { t } = useTranslation()
  const { progress, isScanning } = useScanProgress()
  const [dismissed, setDismissed] = useState(false)

  // Setup cache invalidation (runs automatically)
  useScanCompletionInvalidation()

  // Reset dismissed state when a new scan starts
  useEffect(() => {
    if (isScanning) {
      setDismissed(false)
    }
  }, [isScanning])

  // Don't show if dismissed or not scanning
  if (dismissed || !isScanning) {
    return null
  }

  const isComplete = progress === 100

  return (
    <div className="fixed bottom-4 right-4 z-50 max-w-sm">
      <div className="bg-card border border-border rounded-lg shadow-lg p-4">
        <div className="flex items-start gap-3">
          {/* Icon */}
          <div className="flex-shrink-0 mt-0.5">
            {isComplete ? (
              <CheckCircle2 className="w-5 h-5 text-green-500" />
            ) : (
              <Loader2 className="w-5 h-5 animate-spin text-primary" />
            )}
          </div>

          {/* Content */}
          <div className="flex-1 min-w-0">
            <p className="text-sm font-medium">
              {isComplete
                ? t('library.scanComplete')
                : t('library.scanning')}
            </p>

            {!isComplete && (
              <>
                <p className="text-xs text-muted-foreground mt-1">
                  {t('library.scanningLibrary')}
                </p>

                {/* Progress bar */}
                <div className="mt-2 w-full h-1.5 bg-muted rounded-full overflow-hidden">
                  <div
                    className="h-full bg-primary transition-all duration-300"
                    style={{ width: `${progress}%` }}
                  />
                </div>

                {/* Progress percentage */}
                <p className="text-xs text-muted-foreground mt-1">
                  {Math.round(progress)}%
                </p>
              </>
            )}
          </div>

          {/* Dismiss button */}
          <button
            onClick={() => setDismissed(true)}
            className="flex-shrink-0 p-1 hover:bg-foreground/10 rounded transition-colors"
            aria-label={t('common.close')}
          >
            <X className="w-4 h-4 text-muted-foreground" />
          </button>
        </div>
      </div>
    </div>
  )
}
