import tsParser from '@typescript-eslint/parser';

export default [
  {
    files: ['src/**/*.ts'],
    languageOptions: {
      parser: tsParser,
      parserOptions: { ecmaVersion: 'latest', sourceType: 'module' },
    },
    rules: {
      'no-debugger': 'error',
      'no-constant-binary-expression': 'error',
      'no-duplicate-case': 'error',
      'no-unreachable': 'error',
    },
  },
];
