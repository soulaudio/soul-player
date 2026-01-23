/// <reference types="vitest" />
/// <reference types="@testing-library/jest-dom" />

import type { TestingLibraryMatchers } from '@testing-library/jest-dom/matchers';
import 'vitest';

declare module 'vitest' {
  interface Assertion<T = any> extends TestingLibraryMatchers<T, void> {}
  interface AsymmetricMatchersContaining extends TestingLibraryMatchers {}
}
