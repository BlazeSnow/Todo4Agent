<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { Group, Task } from '../types'
import { NO_GROUP_NAME } from '../types'
import { dateLocale } from '../i18n'
import ArchiveTaskCard from './ArchiveTaskCard.vue'
import InfoTip from './InfoTip.vue'

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

const { t } = useI18n()

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

/** 日期标签：今天 / 昨天 / 其余按界面语言显示完整日期（含星期） */
function dayLabel(d: Date): string {
  const today = new Date()
  const sameDay = (a: Date, b: Date) =>
    a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate()
  const yesterday = new Date(today)
  yesterday.setDate(today.getDate() - 1)
  if (sameDay(d, today)) return t('archive.today')
  if (sameDay(d, yesterday)) return t('archive.yesterday')
  return d.toLocaleDateString(dateLocale.value, { year: 'numeric', month: 'long', day: 'numeric', weekday: 'long' })
}

function groupNameOf(groupId: number): string {
  const g = props.activeGroups.find((x) => x.id === groupId)
  if (!g) return t('groups.groupDeleted')
  return g.name === NO_GROUP_NAME ? t('groups.noGroup') : g.name
}
</script>

<template>
  <div>
    <div class="d-flex align-center mb-4">
      <v-icon icon="mdi-archive-outline" class="mr-2" />
      <h2 class="text-h6">{{ t('archive.title') }}</h2>
      <InfoTip :text="t('archive.hint')" />
      <v-chip v-if="tasks.length > 0" size="small" variant="tonal" class="ml-3">
        {{ t('archive.taskCount', { n: tasks.length }) }}
      </v-chip>
      <v-spacer />
    </div>

    <v-empty
      v-if="tasks.length === 0"
      icon="mdi-archive-outline"
      :title="t('archive.empty')"
      :text="t('archive.emptyHint')"
    />

    <!-- 按归档日期分组：日期标题 + 紧凑扁平卡片列表（无时间线，窄屏省空间） -->
    <div v-for="day in byDay" :key="day.key" class="mb-5">
      <div class="text-subtitle-2 text-medium-emphasis mb-2">{{ day.label }}</div>
      <div class="archive-list">
        <ArchiveTaskCard
          v-for="task in day.items"
          :key="task.id"
          :task="task"
          :group-name="groupNameOf(task.group_id)"
          @restore="$emit('restore', $event)"
          @remove="$emit('remove', $event)"
        />
      </div>
    </div>
  </div>
</template>

<style scoped>
.archive-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
</style>
