import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'

// Production is served under the `/seeki` subpath behind the sg.aurrigo.com Caddy
// edge (which STRIPs the prefix), so Vite must emit /seeki/-prefixed asset URLs.
// The app's /api calls are prefixed via import.meta.env.BASE_URL (see src/lib/api.ts
// apiUrl). Dev stays at '/' so the proxy below keeps working.
export default defineConfig(({ mode }) => ({
  base: mode === 'production' ? '/seeki/' : '/',
  plugins: [svelte()],
  server: {
    proxy: {
      '/api': {
        target: 'http://127.0.0.1:3141',
        changeOrigin: true,
      },
    },
  },
}))
