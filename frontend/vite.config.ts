import { fileURLToPath, URL } from 'node:url'
import tailwindcss from '@tailwindcss/vite'
import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

const uiSrc = fileURLToPath(new URL('../packages/ui/src', import.meta.url))

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: [
      { find: '@kiteframe/ui/styles.css', replacement: `${uiSrc}/styles.css` },
      { find: '@kiteframe/ui', replacement: `${uiSrc}/index.ts` },
    ],
  },
  server: {
    proxy: { '/api': 'http://localhost:8080' },
    fs: { allow: ['..'] },
  },
})
