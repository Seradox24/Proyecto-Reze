import js from '@eslint/js';
import globals from 'globals';
import prettier from 'eslint-config-prettier';
import tseslint from 'typescript-eslint';

export default tseslint.config(
  { ignores: ['target/', 'node_modules/', 'ui/dist/', 'artifacts/', 'src-tauri/gen/'] },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  { files: ['ui/**/*.ts'], languageOptions: { globals: globals.browser } },
  { files: ['*.config.{js,ts}'], languageOptions: { globals: globals.node } },
  {
    // Native smoke scripts run in Node and evaluate snippets inside the WebView.
    files: ['scripts/**/*.mjs'],
    languageOptions: { globals: { ...globals.browser, ...globals.node } },
  },
  {
    // The Playwright suite drives a hand-written IPC double through `window`.
    files: ['tests/**/*.ts'],
    languageOptions: { globals: { ...globals.browser, ...globals.node } },
    rules: { '@typescript-eslint/no-explicit-any': 'off' },
  },
  prettier,
);
