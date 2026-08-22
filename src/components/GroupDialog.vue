<script setup lang="ts">
import { ref, watch } from 'vue'
import type { Group } from '../types'

const props = defineProps<{
  modelValue: boolean
  mode: 'create' | 'rename'
  group: Group | null
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', v: boolean): void
  (e: 'save', name: string): void
}>()

const name = ref('')

watch(
  () => props.modelValue,
  (open) => {
    if (open) name.value = props.group?.name ?? ''
  },
)

function save() {
  const v = name.value.trim()
  if (!v) return
  emit('save', v)
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
      <v-card-title>{{ mode === 'create' ? '新增分组' : '重命名分组' }}</v-card-title>
      <v-card-text>
        <v-text-field v-model="name" label="分组名称" autofocus @keydown.enter="save" />
      </v-card-text>
      <v-card-actions>
        <v-spacer />
        <v-btn variant="text" @click="emit('update:modelValue', false)">取消</v-btn>
        <v-btn color="primary" :disabled="!name.trim()" @click="save">保存</v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>