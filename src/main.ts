import { createApp } from 'vue'
import { createVuetify } from 'vuetify'
import 'vuetify/styles'
import '@mdi/font/css/materialdesignicons.css'
// 代码块统一字体（Agent 接入页配置示例、提示词编辑器）
import '@fontsource/monaspace-neon/400.css'
import App from './App.vue'
import { i18n, locale, vuetifyLocaleOf } from './i18n'

const vuetify = createVuetify({
  locale: {
    // Vuetify 内置组件文案跟随界面语言（运行时切换由 LocaleSwitch 经 useLocale 同步）
    locale: vuetifyLocaleOf(locale.value),
  },
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

createApp(App).use(vuetify).use(i18n).mount('#app')
