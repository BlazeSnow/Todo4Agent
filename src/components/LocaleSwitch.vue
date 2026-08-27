<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useDisplay, useLocale as useVuetifyLocale } from 'vuetify'
import { LOCALE_NAMES, setLocale, type AppLocale } from '../i18n'

withDefaults(defineProps<{ variant?: 'text' | 'tonal' }>(), { variant: 'text' })

const { t, locale } = useI18n()
const current = computed(() => locale.value as AppLocale)
const vuetifyLocale = useVuetifyLocale()
const { width } = useDisplay()
const isSmall = computed(() => width.value < 600)

/** 切换语言：i18n 主文案 + Vuetify 内置文案 + 持久化 */
function pick(l: AppLocale) {
  setLocale(l)
  vuetifyLocale.current.value = l === 'zh-CN' ? 'zhHans' : 'en'
}
</script>

<template>
  <v-menu>
    <template #activator="{ props }">
      <!-- 小屏（手机）切换为 Vuetify 原生图标按钮，保证图标居中；
           icon 属性仅在无默认插槽时渲染图标，文字走 text 属性 -->
      <v-btn
        v-bind="props"
        :variant="variant"
        :icon="isSmall ? 'mdi-translate' : undefined"
        :prepend-icon="isSmall ? undefined : 'mdi-translate'"
        :text="isSmall ? undefined : LOCALE_NAMES[current]"
        :aria-label="t('common.language')"
      />
    </template>
    <v-list density="compact">
      <v-list-item
        v-for="(name, l) in LOCALE_NAMES"
        :key="l"
        :title="name"
        :active="current === l"
        @click="pick(l)"
      />
    </v-list>
  </v-menu>
</template>
