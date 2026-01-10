import eslint from '@eslint/js'
import tseslint from 'typescript-eslint'
import reactHooks from 'eslint-plugin-react-hooks'
import globals from 'globals'

export default tseslint.config(
  // Global ignores - must be first
  {
    ignores: [
      'node_modules/**',
      '.next/**',
      'out/**',
      '*.config.js',
      '*.config.ts',
      '*.config.mjs',
      'contrast-report.mjs',
      'scripts/**',
      'inspect-themes.js',
      'src/wasm/**',  // Generated WASM bindings
    ],
  },
  eslint.configs.recommended,
  ...tseslint.configs.recommended,
  {
    languageOptions: {
      globals: {
        ...globals.browser,
        ...globals.node,
      },
    },
    plugins: {
      'react-hooks': reactHooks,
    },
    rules: {
      ...reactHooks.configs.recommended.rules,
      '@typescript-eslint/no-explicit-any': 'warn',
      '@typescript-eslint/no-unused-vars': ['error', { argsIgnorePattern: '^_' }],
    },
  }
)
