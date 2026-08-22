<script setup lang="ts">
const props = defineProps<{
  modelValue: boolean
  message: string
  title?: string
  confirmText?: string
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', v: boolean): void
  (e: 'confirm'): void
}>()

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
      <v-card-title>{{ title ?? '确认删除' }}</v-card-title>
      <v-card-text>{{ message }}</v-card-text>
      <v-card-actions>
        <v-spacer />
        <v-btn text @click="emit('update:modelValue', false)">取消</v-btn>
        <v-btn color="error" @click="confirm">{{ confirmText ?? '删除' }}</v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>