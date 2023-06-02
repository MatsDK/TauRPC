/** @type {import("eslint").Linter.Config} */
const config = {
  parser: '@typescript-eslint/parser',
  env: {
    node: true,
    browser: true,
  },
  extends: [
    'plugin:@typescript-eslint/recommended',
    'plugin:@typescript-eslint/recommended-requiring-type-checking',
    'eslint:recommended',
    // 'plugin:dprint/recommended',
  ],
  parserOptions: {
    ecmaVersion: 'latest',
    tsconfigRootDir: __dirname,
    project: [
      './tsconfig.json',
      './example/tsconfig.json',
    ],
  },
  rules: {
    'no-unused-vars': 'off',
    '@typescript-eslint/no-unused-vars': [
      'warn', // or "error"
      {
        'argsIgnorePattern': '^_',
        'varsIgnorePattern': '^_',
        'caughtErrorsIgnorePattern': '^_',
      },
    ],
    // 'dprint/dprint': [
    // 	'error',
    // 	{
    // 		config: {},
    // 	},
    // ],
  },
  plugins: [
    '@typescript-eslint',
  ],
  ignorePatterns: [
    'node_modules',
    'dist',
    'taurpc',
    'target',
    'test',
    'example',
  ],
}

module.exports = config
