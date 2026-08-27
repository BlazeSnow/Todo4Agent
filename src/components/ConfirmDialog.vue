<script setup lang="ts">
import { useI18n } from 'vue-i18n'

const props = defineProps<{
  modelValue: boolean
  message: string
  title?: string
  confirmText?: string
  /** 确认按钮颜色（默认 error，用于删除类操作） */
  color?: string
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', v: boolean): void
  (e: 'confirm'): void
}>()

const { t } = useI18n()

function confirm() {
  emit('confirm')
  emit('update:modelValue', false)
}
</script>

<template>
  <v-dialog
    :model-value="modelValue"
    max-width="420"
    @update:model-value="emit('update:modelValue', $event)"
  >
    <v-card>
      <v-card-title>{{ title ?? t('confirm.title') }}</v-card-title>
      <v-card-text>{{ message }}</v-card-text>
      <v-card-actions>
        <v-spacer />
        <v-btn text @click="emit('update:modelValue', false)">{{ t('common.cancel') }}</v-btn>
        <v-btn :color="color ?? 'error'" @click="confirm">{{ confirmText ?? t('common.delete') }}</v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>