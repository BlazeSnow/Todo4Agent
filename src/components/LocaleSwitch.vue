<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useLocale as useVuetifyLocale } from 'vuetify'
import { LOCALE_NAMES, setLocale, type AppLocale } from '../i18n'

withDefaults(defineProps<{ variant?: 'text' | 'tonal' }>(), { variant: 'text' })

const { t, locale } = useI18n()
const current = computed(() => locale.value as AppLocale)
const vuetifyLocale = useVuetifyLocale()

/** 切换语言：i18n 主文案 + Vuetify 内置文案 + 持久化 */
function pick(l: AppLocale) {
  setLocale(l)
  vuetifyLocale.current.value = l === 'zh-CN' ? 'zhHans' : 'en'
}
</script>

<template>
  <v-menu>
    <template #activator="{ props }">
      <v-btn
        v-bind="props"
        :variant="variant"
        prepend-icon="mdi-translate"
        :aria-label="t('common.language')"
      >
        {{ LOCALE_NAMES[current] }}
      </v-btn>
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
