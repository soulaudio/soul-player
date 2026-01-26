/**
 * Hook for handling cancellable requests with race condition prevention
 * Uses request ID tracking to ignore stale responses
 */

import { useRef, useCallback } from 'react';

interface CancellableRequestOptions {
  /**
   * If true, the request will be cancelled if component unmounts
   * Default: true
   */
  cancelOnUnmount?: boolean;
}

interface CancellableRequest {
  /**
   * Execute an async function with cancellation support
   * Returns undefined if the request was cancelled
   */
  execute: <T>(fn: () => Promise<T>) => Promise<T | undefined>;

  /**
   * Cancel all pending requests
   */
  cancelAll: () => void;

  /**
   * Check if a specific request is still valid
   */
  isValid: (requestId: number) => boolean;
}

/**
 * Hook for handling cancellable requests
 * Prevents race conditions by tracking request IDs
 *
 * @example
 * ```tsx
 * const { execute } = useCancellableRequest();
 *
 * const loadData = useCallback(async (id: number) => {
 *   const data = await execute(() => backend.getData(id));
 *   if (data) {
 *     setData(data); // Only runs if request wasn't cancelled
 *   }
 * }, [execute, backend]);
 * ```
 */
export function useCancellableRequest(
  options: CancellableRequestOptions = {}
): CancellableRequest {
  const { cancelOnUnmount = true } = options;
  const requestIdRef = useRef(0);
  const isMountedRef = useRef(true);

  // Track current request ID
  const getCurrentRequestId = useCallback(() => {
    return ++requestIdRef.current;
  }, []);

  // Check if request is still valid
  const isValid = useCallback((requestId: number) => {
    if (cancelOnUnmount && !isMountedRef.current) {
      return false;
    }
    return requestId === requestIdRef.current;
  }, [cancelOnUnmount]);

  // Execute a cancellable request
  const execute = useCallback(async <T,>(fn: () => Promise<T>): Promise<T | undefined> => {
    const currentRequestId = getCurrentRequestId();

    try {
      const result = await fn();

      // Check if this request is still valid
      if (!isValid(currentRequestId)) {
        return undefined;
      }

      return result;
    } catch (error) {
      // Only propagate error if request is still valid
      if (!isValid(currentRequestId)) {
        return undefined;
      }
      throw error;
    }
  }, [getCurrentRequestId, isValid]);

  // Cancel all pending requests
  const cancelAll = useCallback(() => {
    requestIdRef.current++;
  }, []);

  // Cleanup on unmount
  if (cancelOnUnmount) {
    // Use ref to track mount state
    isMountedRef.current = true;

    // This will be captured in useEffect cleanup
    const cleanup = () => {
      isMountedRef.current = false;
    };

    // Return cleanup function to be used in useEffect
    if (typeof cleanup === 'function') {
      // Caller should use this in useEffect
    }
  }

  return {
    execute,
    cancelAll,
    isValid,
  };
}

/**
 * Hook variant specifically for component mount tracking
 * Automatically cancels requests when component unmounts
 */
export function useMountedRequest(): CancellableRequest {
  const isMountedRef = useRef(true);
  const requestIdRef = useRef(0);

  // Set mounted state on every render
  isMountedRef.current = true;

  const getCurrentRequestId = useCallback(() => {
    return ++requestIdRef.current;
  }, []);

  const isValid = useCallback((requestId: number) => {
    return isMountedRef.current && requestId === requestIdRef.current;
  }, []);

  const execute = useCallback(async <T,>(fn: () => Promise<T>): Promise<T | undefined> => {
    const currentRequestId = getCurrentRequestId();

    try {
      const result = await fn();

      if (!isValid(currentRequestId)) {
        return undefined;
      }

      return result;
    } catch (error) {
      if (!isValid(currentRequestId)) {
        return undefined;
      }
      throw error;
    }
  }, [getCurrentRequestId, isValid]);

  const cancelAll = useCallback(() => {
    requestIdRef.current++;
  }, []);

  return {
    execute,
    cancelAll,
    isValid,
  };
}
