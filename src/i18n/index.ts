import { computed } from 'vue'
import { createI18n } from 'vue-i18n'
import zhCN from './zh-CN'
import enUS from './en-US'

export type AppLocale = 'zh-CN' | 'en-US'

/** 语言菜单里的显示名（各语言以自身名称展示） */
export const LOCALE_NAMES: Record<AppLocale, string> = {
  'zh-CN': '简体中文',
  'en-US': 'English',
}

const STORAGE_KEY = 'todo4agent-lang'

/** 初始语言：localStorage 记忆 > 浏览器/系统语言 > 中文 */
function detectLocale(): AppLocale {
  const saved = localStorage.getItem(STORAGE_KEY)
  if (saved === 'zh-CN' || saved === 'en-US') return saved
  return navigator.language.toLowerCase().startsWith('zh') ? 'zh-CN' : 'en-US'
}

export const i18n = createI18n({
  legacy: false,
  locale: detectLocale(),
  fallbackLocale: 'zh-CN',
  messages: {
    'zh-CN': zhCN,
    'en-US': enUS,
  },
})

/** 当前语言（可写 ref，切换用 setLocale） */
export const locale = i18n.global.locale

/** 日期/时间格式化所用 BCP-47 区域（跟随界面语言） */
export const dateLocale = computed(() => (locale.value === 'zh-CN' ? 'zh-CN' : 'en-US'))

/** 标题排序所用区域 */
export const sortLocale = computed(() => (locale.value === 'zh-CN' ? 'zh-Hans-CN' : 'en'))

/** Vuetify 内置文案的语言标识 */
export function vuetifyLocaleOf(l: string): string {
  return l === 'zh-CN' ? 'zhHans' : 'en'
}

/** 切换界面语言并持久化（Vuetify 文案由调用方经 useLocale 同步） */
export function setLocale(l: AppLocale) {
  locale.value = l
  localStorage.setItem(STORAGE_KEY, l)
  document.documentElement.lang = l
}

document.documentElement.lang = locale.value
