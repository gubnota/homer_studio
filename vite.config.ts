import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import { resolve } from 'node:path'

export default defineConfig({
  root: resolve('src/renderer'),
  plugins: [react()],
  clearScreen: false,
  server: {
    host: '127.0.0.1',
    port: 1420,
    strictPort: true
  },
  envPrefix: ['VITE_', 'TAURI_'],
  build: {
    outDir: resolve('dist'),
    emptyOutDir: true,
    target: 'safari13',
    minify: process.env.TAURI_DEBUG ? false : 'esbuild',
    sourcemap: Boolean(process.env.TAURI_DEBUG)
  }
})
