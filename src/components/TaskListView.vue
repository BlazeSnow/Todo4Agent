<script setup lang="ts">
import type { Task } from '../types'

defineProps<{
  tasks: Task[]
  loading: boolean
  /** 当前分组名（未选择时为 null） */
  groupName: string | null
}>()

defineEmits<{
  (e: 'create'): void
  (e: 'edit', task: Task): void
  (e: 'toggle', task: Task): void
  (e: 'remove', task: Task): void
}>()

function formatDue(iso: string): string {
  const d = new Date(iso)
  if (isNaN(d.getTime())) return iso
  return d.toLocaleString('zh-CN', { dateStyle: 'medium', timeStyle: 'short' })
}

function overdue(task: Task): boolean {
  if (!task.due_at || task.status === 'done') return false
  const d = new Date(task.due_at)
  return !isNaN(d.getTime()) && d.getTime() < Date.now()
}
</script>

<template>
  <div>
    <div class="d-flex align-center mb-4">
      <v-icon icon="mdi-folder-outline" class="mr-2" />
      <h2 class="text-h6">{{ groupName ?? '未选择分组' }}</h2>
      <v-spacer />
      <v-btn color="primary" prepend-icon="mdi-plus" @click="$emit('create')">
        新建任务
      </v-btn>
    </div>

    <v-alert v-if="!groupName" type="info" text="请先在左侧创建分组" />

    <v-progress-linear v-if="loading" indeterminate class="mb-4" />

    <v-card
      v-for="task in tasks"
      :key="task.id"
      class="mb-2"
      variant="outlined"
      :class="{ 'opacity-60': task.status === 'done' }"
    >
      <v-list-item>
        <template #prepend>
          <v-checkbox-btn
            :model-value="task.status === 'done'"
            color="success"
            @update:model-value="$emit('toggle', task)"
          />
        </template>
        <v-list-item-title
          :class="{ 'text-decoration-line-through': task.status === 'done' }"
        >
          {{ task.title }}
        </v-list-item-title>
        <v-list-item-subtitle v-if="task.description" class="text-pre-wrap">
          {{ task.description }}
        </v-list-item-subtitle>
        <v-list-item-subtitle v-if="task.due_at" class="mt-1">
          <v-chip size="small" variant="tonal" :color="overdue(task) ? 'error' : 'default'">
            <v-icon start icon="mdi-calendar" size="small" />
            {{ formatDue(task.due_at) }}
          </v-chip>
        </v-list-item-subtitle>
        <template #append>
          <v-btn icon="mdi-pencil" size="small" variant="text" @click="$emit('edit', task)" />
          <v-btn
            icon="mdi-delete"
            size="small"
            variant="text"
            color="error"
            @click="$emit('remove', task)"
          />
        </template>
      </v-list-item>
    </v-card>

    <v-empty
      v-if="!loading && groupName && tasks.length === 0"
      icon="mdi-inbox-outline"
      title="暂无任务"
      text="点击右上角「新建任务」，或让 Agent 通过 MCP 添加"
    />
  </div>
</template>