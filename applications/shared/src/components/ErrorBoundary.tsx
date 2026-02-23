/**
 * ErrorBoundary - Catches React errors and displays fallback UI
 * Prevents the entire app from crashing when a component throws an error
 */

import { Component, type ReactNode, type ErrorInfo } from 'react'
import { AlertTriangle } from 'lucide-react'
import { debug } from '../utils/debug';
import i18n from '../i18n';

interface ErrorBoundaryProps {
  children: ReactNode
  /** Optional fallback UI to display when an error occurs */
  fallback?: (error: Error, errorInfo: ErrorInfo, reset: () => void) => ReactNode
  /** Optional callback when an error is caught */
  onError?: (error: Error, errorInfo: ErrorInfo) => void
  /** Optional name for this boundary (used in error logging) */
  name?: string
}

interface ErrorBoundaryState {
  hasError: boolean
  error: Error | null
  errorInfo: ErrorInfo | null
}

export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  constructor(props: ErrorBoundaryProps) {
    super(props)
    this.state = {
      hasError: false,
      error: null,
      errorInfo: null,
    }
  }

  static getDerivedStateFromError(error: Error): Partial<ErrorBoundaryState> {
    // Update state so the next render shows the fallback UI
    return {
      hasError: true,
      error,
    }
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo): void {
    // Log error details
    const boundaryName = this.props.name || 'ErrorBoundary'
    debug.error(`[${boundaryName}] Caught error:`, error)
    debug.error(`[${boundaryName}] Error info:`, errorInfo)

    // Update state with error info
    this.setState({ errorInfo })

    // Call optional error handler
    this.props.onError?.(error, errorInfo)
  }

  private reset = (): void => {
    this.setState({
      hasError: false,
      error: null,
      errorInfo: null,
    })
  }

  render(): ReactNode {
    const { hasError, error, errorInfo } = this.state
    const { children, fallback } = this.props

    if (hasError && error && errorInfo) {
      // Use custom fallback if provided
      if (fallback) {
        return fallback(error, errorInfo, this.reset)
      }

      // Default fallback UI
      return (
        <div className="flex items-center justify-center min-h-[400px] p-6">
          <div className="max-w-lg text-center">
            <div className="flex justify-center mb-4">
              <AlertTriangle className="w-12 h-12 text-destructive" />
            </div>
            <h2 className="text-2xl font-bold mb-2">{i18n.t('errorBoundary.title')}</h2>
            <p className="text-muted-foreground mb-4">
              {i18n.t('errorBoundary.description')}
            </p>

            {/* Error details (collapsed by default) */}
            <details className="text-left mb-4">
              <summary className="cursor-pointer text-sm font-medium hover:underline">
                {i18n.t('errorBoundary.showDetails')}
              </summary>
              <div className="mt-2 p-3 bg-muted rounded-lg text-xs font-mono overflow-auto max-h-48">
                <div className="mb-2">
                  <strong>{i18n.t('errorBoundary.errorLabel')}</strong> {error.message}
                </div>
                {error.stack && (
                  <div className="mb-2">
                    <strong>{i18n.t('errorBoundary.stackLabel')}</strong>
                    <pre className="whitespace-pre-wrap break-words">{error.stack}</pre>
                  </div>
                )}
                {errorInfo.componentStack && (
                  <div>
                    <strong>{i18n.t('errorBoundary.componentStackLabel')}</strong>
                    <pre className="whitespace-pre-wrap break-words">{errorInfo.componentStack}</pre>
                  </div>
                )}
              </div>
            </details>

            {/* Actions */}
            <div className="flex gap-3 justify-center">
              <button
                onClick={this.reset}
                className="px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:opacity-[var(--hover-button-opacity)] transition-all duration-[var(--transition-duration)]"
              >
                {i18n.t('errorBoundary.tryAgain')}
              </button>
              <button
                onClick={() => window.location.reload()}
                className="px-4 py-2 bg-secondary text-secondary-foreground rounded-lg hover:opacity-[var(--hover-button-opacity)] transition-all duration-[var(--transition-duration)]"
              >
                {i18n.t('errorBoundary.reloadPage')}
              </button>
            </div>
          </div>
        </div>
      )
    }

    return children
  }
}

/**
 * Hook to manually trigger error boundary
 * Useful for async errors that occur outside of render
 */
export function useErrorBoundary() {
  const timerRef = { current: null as NodeJS.Timeout | null }

  const throwError = (error: Error): void => {
    // Clear any pending timer
    if (timerRef.current) {
      clearTimeout(timerRef.current)
    }

    // Throw error in next tick to trigger error boundary
    timerRef.current = setTimeout(() => {
      timerRef.current = null
      throw error
    }, 0)
  }

  return { throwError }
}
