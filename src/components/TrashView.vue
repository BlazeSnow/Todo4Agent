<script setup lang="ts">
import type { Group, Task } from '../types'

defineProps<{
  /** 回收站中的分组 */
  groups: Group[]
  /** 回收站中的任务 */
  tasks: Task[]
  /** 当前存在的分组（用于展示任务所属组名，找不到说明组已删除） */
  activeGroups: Group[]
}>()

defineEmits<{
  (e: 'restore', kind: 'group' | 'task', id: number): void
  (e: 'purge', kind: 'group' | 'task', id: number): void
  (e: 'empty'): void
}>()

function formatTime(iso: string): string {
  const d = new Date(iso)
  if (isNaN(d.getTime())) return iso
  return d.toLocaleString('zh-CN', { dateStyle: 'short', timeStyle: 'short' })
}

function groupNameOf(groupId: number, activeGroups: Group[]): string {
  const g = activeGroups.find((x) => x.id === groupId)
  return g ? g.name : '（分组已删除）'
}
</script>

<template>
  <div>
    <div class="d-flex align-center mb-4">
      <v-icon icon="mdi-trash-can-outline" class="mr-2" />
      <h2 class="text-h6">回收站</h2>
      <v-spacer />
      <v-btn
        v-if="groups.length > 0 || tasks.length > 0"
        color="error"
        variant="tonal"
        prepend-icon="mdi-delete-sweep-outline"
        @click="$emit('empty')"
      >
        清空回收站
      </v-btn>
    </div>

    <v-empty
      v-if="groups.length === 0 && tasks.length === 0"
      icon="mdi-trash-can-outline"
      title="回收站为空"
      text="删除的任务和分组会在这里保留，可恢复或彻底删除"
    />

    <template v-if="groups.length > 0">
      <div class="text-subtitle-2 text-medium-emphasis mb-2">已删除的分组</div>
      <v-card v-for="group in groups" :key="group.id" class="mb-2" variant="outlined">
        <v-list-item>
          <template #prepend>
            <v-icon icon="mdi-folder-remove-outline" />
          </template>
          <v-list-item-title>{{ group.name }}</v-list-item-title>
          <v-list-item-subtitle>删除于 {{ formatTime(group.deleted_at!) }}</v-list-item-subtitle>
          <template #append>
            <v-btn
              icon="mdi-restore"
              size="small"
              variant="text"
              title="恢复分组及其任务"
              @click="$emit('restore', 'group', group.id)"
            />
            <v-btn
              icon="mdi-delete-forever"
              size="small"
              variant="text"
              color="error"
              title="彻底删除分组及其任务"
              @click="$emit('purge', 'group', group.id)"
            />
          </template>
        </v-list-item>
      </v-card>
    </template>

    <template v-if="tasks.length > 0">
      <div class="text-subtitle-2 text-medium-emphasis mb-2 mt-4">已删除的任务</div>
      <v-card v-for="task in tasks" :key="task.id" class="mb-2" variant="outlined">
        <v-list-item>
          <template #prepend>
            <v-icon icon="mdi-checkbox-marked-circle-outline" color="grey" />
          </template>
          <v-list-item-title
            :class="{ 'text-decoration-line-through': task.status === 'done' }"
          >
            {{ task.title }}
          </v-list-item-title>
          <v-list-item-subtitle>
            {{ groupNameOf(task.group_id, activeGroups) }}
            · 删除于 {{ formatTime(task.deleted_at!) }}
          </v-list-item-subtitle>
          <template #append>
            <v-btn
              icon="mdi-restore"
              size="small"
              variant="text"
              title="恢复任务"
              @click="$emit('restore', 'task', task.id)"
            />
            <v-btn
              icon="mdi-delete-forever"
              size="small"
              variant="text"
              color="error"
              title="彻底删除任务"
              @click="$emit('purge', 'task', task.id)"
            />
          </template>
        </v-list-item>
      </v-card>
    </template>
  </div>
</template>