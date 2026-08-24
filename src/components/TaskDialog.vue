<script setup lang="ts">
import { ref, watch } from 'vue'
import type { Group, Task, TaskInput } from '../types'
import { onEnterSubmit } from '../ime'

const props = defineProps<{
  modelValue: boolean
  task: Task | null
  groups: Group[]
  /** 新建任务时默认选中的分组（当前界面上选中的分组） */
  defaultGroupId: number | null
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', v: boolean): void
  (e: 'save', input: TaskInput): void
}>()

const groupId = ref<number | null>(null)
const title = ref('')
const description = ref('')
const dueLocal = ref('')

/** ISO 时间 → 本地 datetime-local 输入值 */
function isoToLocal(iso: string): string {
  const d = new Date(iso)
  if (isNaN(d.getTime())) return ''
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`
}

watch(
  () => props.modelValue,
  (open) => {
    if (!open) return
    const t = props.task
    title.value = t?.title ?? ''
    description.value = t?.description ?? ''
    dueLocal.value = t?.due_at ? isoToLocal(t.due_at) : ''
    groupId.value = t?.group_id ?? props.defaultGroupId ?? props.groups[0]?.id ?? null
  },
)

function save() {
  const g = groupId.value
  if (g == null || !title.value.trim()) return
  const input: TaskInput = {
    group_id: g,
    title: title.value.trim(),
    description: description.value.trim(),
    due_at: dueLocal.value ? new Date(dueLocal.value).toISOString() : null,
  }
  emit('save', input)
  emit('update:modelValue', false)
}
</script>

<template>
  <v-dialog
    :model-value="modelValue"
    max-width="560"
    @update:model-value="emit('update:modelValue', $event)"
  >
    <v-card>
      <v-card-title>{{ task ? '编辑任务' : '新建任务' }}</v-card-title>
      <v-card-text>
        <v-select
          v-model="groupId"
          :items="groups"
          item-title="name"
          item-value="id"
          label="分组"
          :disabled="groups.length === 0"
          class="mb-2"
        />
        <v-text-field v-model="title" label="任务标题" required autofocus @keydown.enter="onEnterSubmit($event, save)" />
        <v-textarea v-model="description" label="详细说明（可选）" rows="3" auto-grow />
        <v-text-field v-model="dueLocal" label="截止时间（可选）" type="datetime-local" clearable />
      </v-card-text>
      <v-card-actions>
        <v-spacer />
        <v-btn variant="text" @click="emit('update:modelValue', false)">取消</v-btn>
        <v-btn color="primary" :disabled="!title.trim() || groupId == null" @click="save">
          保存
        </v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>