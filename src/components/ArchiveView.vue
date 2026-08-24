<script setup lang="ts">
import { computed } from 'vue'
import type { Group, Task } from '../types'

const props = defineProps<{
  /** 已归档的任务（后端按归档时间倒序返回） */
  tasks: Task[]
  /** 当前存在的分组（展示任务所属组名） */
  activeGroups: Group[]
}>()

const emit = defineEmits<{
  (e: 'restore', task: Task): void
  (e: 'remove', task: Task): void
}>()

/** 按归档日期（本地时区）分组，保持倒序：[{ key, label, items }] */
const byDay = computed(() => {
  const days: { key: string; label: string; items: Task[] }[] = []
  for (const t of props.tasks) {
    const d = new Date(t.archived_at ?? '')
    if (isNaN(d.getTime())) continue
    const key = `${d.getFullYear()}-${d.getMonth() + 1}-${d.getDate()}`
    let day = days.find((x) => x.key === key)
    if (!day) {
      day = { key, label: dayLabel(d), items: [] }
      days.push(day)
    }
    day.items.push(t)
  }
  return days
})

/** 日期标签：今天 / 昨天 / 其余显示中文完整日期（含星期） */
function dayLabel(d: Date): string {
  const today = new Date()
  const sameDay = (a: Date, b: Date) =>
    a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate()
  const yesterday = new Date(today)
  yesterday.setDate(today.getDate() - 1)
  if (sameDay(d, today)) return '今天'
  if (sameDay(d, yesterday)) return '昨天'
  return d.toLocaleDateString('zh-CN', { year: 'numeric', month: 'long', day: 'numeric', weekday: 'long' })
}

function timeOf(iso: string): string {
  const d = new Date(iso)
  if (isNaN(d.getTime())) return iso
  return d.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' })
}

function groupNameOf(groupId: number): string {
  const g = props.activeGroups.find((x) => x.id === groupId)
  return g ? g.name : '（分组已删除）'
}
</script>

<template>
  <div>
    <div class="d-flex align-center mb-4">
      <v-icon icon="mdi-archive-outline" class="mr-2" />
      <h2 class="text-h6">归档</h2>
      <v-chip v-if="tasks.length > 0" size="small" variant="tonal" class="ml-3">
        {{ tasks.length }} 个任务
      </v-chip>
      <v-spacer />
      <span class="text-caption text-medium-emphasis">在任务卡片菜单中选择「归档」，任务会按时间保留在这里</span>
    </div>

    <v-empty
      v-if="tasks.length === 0"
      icon="mdi-archive-outline"
      title="归档为空"
      text="完成的任务可在卡片菜单中归档，归档后按时间线保留在这里，可随时恢复"
    />

    <!-- 按归档日期分组的时间线：每天一个日期标题，组内任务沿时间线排列 -->
    <div v-for="day in byDay" :key="day.key" class="mb-6">
      <div class="text-subtitle-2 text-medium-emphasis mb-3">{{ day.label }}</div>
      <v-timeline density="compact" side="end" line-inset="8">
        <v-timeline-item
          v-for="task in day.items"
          :key="task.id"
          size="x-small"
          :dot-color="task.status === 'done' ? 'primary' : 'grey'"
          :icon="task.status === 'done' ? 'mdi-check' : undefined"
        >
          <div class="task-item archive-item">
            <div class="task-main">
              <div class="task-title" :class="{ struck: task.status === 'done' }">
                {{ task.title }}
                <span class="archive-time text-medium-emphasis">{{ timeOf(task.archived_at!) }}</span>
              </div>
              <div v-if="task.description" class="task-desc">{{ task.description }}</div>
              <div class="task-pills">
                <span class="task-pill">
                  <i class="mdi mdi-folder-outline"></i>
                  {{ groupNameOf(task.group_id) }}
                </span>
              </div>
            </div>
            <div class="task-actions">
              <v-btn
                icon="mdi-package-up-outline"
                size="small"
                variant="text"
                title="取消归档，回到原清单"
                @click="$emit('restore', task)"
              />
              <v-btn
                icon="mdi-delete"
                size="small"
                variant="text"
                color="error"
                title="移入回收站"
                @click="$emit('remove', task)"
              />
            </div>
          </div>
        </v-timeline-item>
      </v-timeline>
    </div>
  </div>
</template>

<style src="../styles/task-card.css"></style>

<style scoped>
.archive-item {
  padding: 10px 14px;
}
.archive-time {
  font-size: 12px;
  font-weight: 400;
  margin-left: 8px;
}
</style>
