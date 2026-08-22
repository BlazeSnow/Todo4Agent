import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import vuetify from 'vite-plugin-vuetify'

// 开发端口固定 3001，/api 代理到后端（后端固定监听 3000，含生产 WebUI）
export default defineConfig({
  plugins: [vue(), vuetify({ autoImport: true })],
  server: {
    port: 3001,
    strictPort: true,
    // 忽略 Rust 构建产物，避免 cargo 编译写入时与文件监控冲突（Windows EBUSY）
    watch: {
      ignored: ['**/src-tauri/target/**', '**/src-tauri/gen/**', '**/dist/**'],
    },
    proxy: {
      '/api': {
        target: 'http://127.0.0.1:3000',
        changeOrigin: true,
      },
    },
  },
})