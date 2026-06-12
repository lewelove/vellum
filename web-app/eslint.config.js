import sveltePlugin from 'eslint-plugin-svelte';
import svelteParser from 'svelte-eslint-parser';

export default [
  ...sveltePlugin.configs['flat/recommended'],
  ...sveltePlugin.configs['flat/prettier'],
  {
    files: ['**/*.svelte'],
    languageOptions: {
      parser: svelteParser,
    },
    rules: {
      'svelte/button-has-type': 'error',
      'svelte/no-unused-svelte-ignore': 'error',
      'svelte/valid-compile': 'error',
      'svelte/no-at-html-tags': 'warn',
      'svelte/infinite-reactive-loop': 'error',
      'svelte/no-dupe-use-directives': 'error'
    }
  }
];
