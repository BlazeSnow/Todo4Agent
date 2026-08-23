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

/** 截止时间格式与普通清单一致（task-card 配套展示辅助） */
function formatDue(iso: string): string {
  const d = new Date(iso)
  if (isNaN(d.getTime())) return iso
  return d.toLocaleString('zh-CN', { dateStyle: 'medium', timeStyle: 'short' })
}

function groupNameOf(groupId: number, activeGroups: Group[]): string {
  const g = activeGroups.find((x) => x.id === groupId)
  return g ? g.name : '（分组已删除）'
}

/** 未完成且已过期（与普通清单的判定一致） */
function overdue(task: Task): boolean {
  if (!task.due_at || task.status === 'done') return false
  const d = new Date(task.due_at)
  return !isNaN(d.getTime()) && d.getTime() < Date.now()
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
      <!-- 与普通清单同一套卡片样式（styles/task-card.css），左端为状态图标、
           右端为恢复/彻底删除，信息 pill 展示所属分组、删除时间与截止时间 -->
      <div
        v-for="task in tasks"
        :key="task.id"
        class="task-item"
        :class="{ done: task.status === 'done' }"
      >
        <v-icon
          :icon="task.status === 'done' ? 'mdi-check-circle' : 'mdi-checkbox-blank-circle-outline'"
          size="18"
          class="task-lead"
          color="grey"
        />
        <div class="task-main">
          <div class="task-title" :class="{ struck: task.status === 'done' }">
            {{ task.title }}
          </div>
          <div v-if="task.description" class="task-desc">{{ task.description }}</div>
          <div class="task-pills">
            <span class="task-pill">
              <i class="mdi mdi-folder-outline"></i>
              {{ groupNameOf(task.group_id, activeGroups) }}
            </span>
            <span class="task-pill">
              <i class="mdi mdi-trash-can-outline"></i>
              删除于 {{ formatTime(task.deleted_at!) }}
            </span>
            <span v-if="task.due_at" class="task-pill" :class="{ overdue: overdue(task) }">
              <i class="mdi mdi-calendar"></i>
              {{ formatDue(task.due_at) }}
            </span>
          </div>
        </div>
        <div class="task-actions">
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
        </div>
      </div>
    </template>
  </div>
</template>

<style src="../styles/task-card.css"></style>