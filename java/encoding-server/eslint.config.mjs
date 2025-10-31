import eslint from '@eslint/js';
import nPlugin from 'eslint-plugin-n';
import globals from 'globals';

export default [
  eslint.configs.recommended,
  {
    plugins: {
      n: nPlugin,
    },
    languageOptions: {
      globals: {
        ...globals.node,
      },
    },
    rules: {
      'n/prefer-node-protocol': 'error',
    },
  },
  {
    ignores: ['node_modules/**', 'cache/**'],
  },
];
