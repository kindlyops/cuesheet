import { defineConfig } from 'vite';

export default defineConfig({
  base: '/cuesheet/',
  build: {
    target: 'es2022',
    sourcemap: false,
  },
});
