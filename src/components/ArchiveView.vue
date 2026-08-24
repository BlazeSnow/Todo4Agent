<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { Group, Task } from '../types'
import { NO_GROUP_NAME } from '../types'
import { dateLocale } from '../i18n'

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

function timeOf(iso: string): string {
  const d = new Date(iso)
  if (isNaN(d.getTime())) return iso
  return d.toLocaleTimeString(dateLocale.value, { hour: '2-digit', minute: '2-digit' })
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
      <v-chip v-if="tasks.length > 0" size="small" variant="tonal" class="ml-3">
        {{ t('archive.taskCount', { n: tasks.length }) }}
      </v-chip>
      <v-spacer />
      <span class="text-caption text-medium-emphasis">{{ t('archive.hint') }}</span>
    </div>

    <v-empty
      v-if="tasks.length === 0"
      icon="mdi-archive-outline"
      :title="t('archive.empty')"
      :text="t('archive.emptyHint')"
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
          <!-- 无界扁平条目：无卡片边框/底色，标题、分组、时间单行排布 -->
          <div class="archive-row">
            <span class="archive-title" :class="{ struck: task.status === 'done' }">
              {{ task.title }}
            </span>
            <span class="archive-group">
              <i class="mdi mdi-folder-outline"></i>
              {{ groupNameOf(task.group_id) }}
            </span>
            <span class="archive-time">{{ timeOf(task.archived_at!) }}</span>
            <v-menu location="bottom right">
              <template #activator="{ props }">
                <v-btn
                  v-bind="props"
                  icon="mdi-dots-horizontal"
                  size="small"
                  variant="text"
                  :aria-label="t('common.moreActions', { name: task.title })"
                />
              </template>
              <v-list density="compact">
                <v-list-item
                  prepend-icon="mdi-archive-arrow-up-outline"
                  :title="t('archive.unarchive')"
                  :subtitle="t('archive.unarchiveSubtitle')"
                  @click="$emit('restore', task)"
                />
                <v-list-item
                  prepend-icon="mdi-delete"
                  :title="t('archive.moveToTrash')"
                  color="error"
                  @click="$emit('remove', task)"
                />
              </v-list>
            </v-menu>
            <div v-if="task.description" class="archive-desc">{{ task.description }}</div>
          </div>
        </v-timeline-item>
      </v-timeline>
    </div>
  </div>
</template>

<style scoped>
/* Vuetify 竖向 side-end 时间线的 body 列为 auto 且 justify-self:flex-start，
   条目会收缩到内容宽度；列改为 1fr、条目 stretch 让其占满右侧全部宽度。
   Vuetify 对奇数项还有一条含 :nth-child 与两个 :not 的 7 级 class 规则
   强制 flex-start，逐级拼选择器不可维护，用 !important 压制 */
.v-timeline--vertical.v-timeline--density-compact.v-timeline--side-end {
  grid-template-columns: 0 min-content 1fr;
}
:deep(.v-timeline-item__body) {
  justify-self: stretch !important;
  padding-inline-start: 12px !important;
}

/* 无界扁平条目：单行（标题 + 分组 + 时间 + 操作），有描述时另起一行 */
.archive-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 2px 12px;
  padding: 4px 0;
}
.archive-title {
  flex: 1 1 auto;
  min-width: 160px;
  font-size: 14px;
  font-weight: 500;
  line-height: 1.4;
  word-break: break-word;
}
.archive-title.struck {
  text-decoration: line-through;
  color: rgba(var(--v-theme-on-surface), 0.45);
}
.archive-group,
.archive-time {
  flex-shrink: 0;
  font-size: 12px;
  color: rgba(var(--v-theme-on-surface), 0.55);
}
.archive-group {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}
.archive-group .mdi {
  font-size: 13px;
}
.archive-desc {
  flex-basis: 100%;
  font-size: 12px;
  line-height: 1.5;
  color: rgba(var(--v-theme-on-surface), 0.55);
  white-space: pre-wrap;
  word-break: break-word;
}
</style>
