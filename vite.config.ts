import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import vuetify from 'vite-plugin-vuetify'

// 开发端口固定 3001，/api 代理到后端（后端固定监听 3000，含生产 WebUI）
export default defineConfig({
  plugins: [vue(), vuetify({ autoImport: true })],
  server: {
    port: 3001,
    strictPort: true,
    proxy: {
      '/api': {
        target: 'http://127.0.0.1:3000',
        changeOrigin: true,
      },
    },
  },
})