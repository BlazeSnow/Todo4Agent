import { createApp } from 'vue'
import { createVuetify } from 'vuetify'
import 'vuetify/styles'
import '@mdi/font/css/materialdesignicons.css'
import App from './App.vue'

const vuetify = createVuetify({
  theme: {
    // 跟随系统深浅色设置（Vuetify 内置监听 prefers-color-scheme）
    defaultTheme: 'system',
    themes: {
      light: {
        colors: {
          primary: '#00a862',
          // 深绿文字保证在主题绿底上的对比度
          'on-primary': '#00332a',
        },
      },
      dark: {
        colors: {
          primary: '#00a862',
          'on-primary': '#00332a',
        },
      },
    },
  },
  icons: {
    defaultSet: 'mdi',
  },
})

createApp(App).use(vuetify).mount('#app')