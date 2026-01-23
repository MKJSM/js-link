import { defineConfig } from 'vite';

export default defineConfig({
  base: '/static/', // Base path for assets when served by backend
  build: {
    outDir: 'dist',
    assetsDir: 'assets',
    emptyOutDir: true,
  },
  resolve: {
    alias: {
      jsonlint: 'jsonlint/web/jsonlint.js',
    },
  },
  server: {
    proxy: {
      // Proxy API requests to backend during dev
      '/api': {
        target: 'http://127.0.0.1:3000',
        changeOrigin: true,
      },
      '/ws': {
        target: 'ws://127.0.0.1:3000',
        ws: true,
      }
    }
  }
});