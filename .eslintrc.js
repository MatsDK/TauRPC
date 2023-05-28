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
    // 'dprint/dprint': [
    // 	'error',
    // 	{
    // 		config: {},
    // 	},
    // ],
    '@typescript-eslint/no-unused-vars': [
      'error',
      {
        argsIgnorePattern: '^_',
        varsIgnorePattern: '^_',
        caughtErrorsIgnorePattern: '^_',
      },
    ],
  },
  plugins: [
    '@typescript-eslint',
  ],
  ignorePatterns: ['node_modules', 'dist', 'taurpc', 'target', 'test'],
}

module.exports = config
