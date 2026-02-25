import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import { resolve } from 'path'

export default defineConfig({
  // Root is the project root (one level up from src-tauri/) so Vite finds index.html
  root: resolve(__dirname, '..'),
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: { '@': resolve(__dirname, '../src') },
  },
  server: {
    port: 1420,
    strictPort: true,
  },
  build: {
    outDir: resolve(__dirname, '../dist'),
    emptyOutDir: true,
  },
})
