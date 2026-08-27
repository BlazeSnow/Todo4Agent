<script setup lang="ts">
import { useI18n } from 'vue-i18n'

withDefaults(defineProps<{ text?: string; maxWidth?: number }>(), {
  text: undefined,
  maxWidth: 340,
})

const { t } = useI18n()
</script>

<!-- 说明性文字统一收纳为 ⓘ 悬停提示：鼠标移上 / 键盘聚焦（触屏点按触发
     hover 模拟）显示详情，避免界面出现大段说明文本 -->
<template>
  <v-tooltip location="bottom" :max-width="maxWidth">
    <template #activator="{ props }">
      <v-icon
        v-bind="props"
        icon="mdi-information-outline"
        size="small"
        class="info-tip"
        tabindex="0"
        :aria-label="t('common.moreInfo')"
      />
    </template>
    <slot>{{ text }}</slot>
  </v-tooltip>
</template>

<style scoped>
.info-tip {
  /* 与左侧标题文字保持固定间距，无需各使用处单独加 margin */
  margin-left: 8px;
  cursor: help;
  opacity: 0.7;
}
.info-tip:hover,
.info-tip:focus-visible {
  opacity: 1;
}
</style>
