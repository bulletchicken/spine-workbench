// ESLint for every TypeScript file in the repository.

import js from '@eslint/js';
import reactHooks from 'eslint-plugin-react-hooks';
import { defineConfig } from 'eslint/config';
import tseslint from 'typescript-eslint';

export default defineConfig(
  {
    // Written by tools, never edited by hand; or not ours.
    ignores: [
      '**/node_modules/',
      '**/dist/',
      'target/',
      'local-only/',
      'app/src/shapes/generated/',
      'app/src/engine-room/wasm-build/',
      'proving-ground/test-results/',
    ],
  },
  js.configs.recommended,
  tseslint.configs.strictTypeChecked,
  {
    languageOptions: {
      parserOptions: {
        projectService: true,
        tsconfigRootDir: import.meta.dirname,
      },
    },
  },
  {
    // This file is plain JavaScript outside every TypeScript project.
    files: ['**/*.js'],
    extends: [tseslint.configs.disableTypeChecked],
  },
  {
    files: ['app/src/**/*.{ts,tsx}'],
    extends: [reactHooks.configs.flat['recommended-latest']],
  },
);
