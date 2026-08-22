import { createApp } from 'vue'
import { createVuetify } from 'vuetify'
import 'vuetify/styles'
import '@mdi/font/css/materialdesignicons.css'
import App from './App.vue'

const vuetify = createVuetify({
  theme: {
    defaultTheme: 'light',
    themes: {
      light: {
        colors: {
          primary: '#03fc8c',
          // 浅绿底上使用深色文字保证对比度
          'on-primary': '#00332a',
          // 主按钮用加深版主题绿（避免过亮）
          accentDeep: '#00a862',
          'on-accentDeep': '#ffffff',
        },
      },
    },
  },
  icons: {
    defaultSet: 'mdi',
  },
})

createApp(App).use(vuetify).mount('#app')