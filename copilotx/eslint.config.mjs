import js from '@eslint/js'
import tseslint from 'typescript-eslint'
import reactHooks from 'eslint-plugin-react-hooks'

export default tseslint.config(
  { ignores: ['dist', 'out', 'sidecar', 'scripts'] },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    files: ['src/renderer/src/**/*.{ts,tsx}'],
    plugins: { 'react-hooks': reactHooks },
    rules: {
      'react-hooks/rules-of-hooks': 'error',
      'react-hooks/exhaustive-deps': 'warn'
    }
  },
  {
    files: ['src/main/**/*.{ts}', 'src/preload/**/*.{ts}'],
    rules: {
      'no-console': 'warn'
    }
  }
)
