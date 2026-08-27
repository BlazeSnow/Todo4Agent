<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { Task } from '../types'
import { dateLocale } from '../i18n'

defineProps<{
  task: Task
  /** 所属分组显示名（父组件已处理「无分组」等翻译） */
  groupName: string
}>()

defineEmits<{
  (e: 'restore', task: Task): void
  (e: 'remove', task: Task): void
}>()

const { t } = useI18n()

function timeOf(iso: string): string {
  const d = new Date(iso)
  if (isNaN(d.getTime())) return iso
  return d.toLocaleTimeString(dateLocale.value, { hour: '2-digit', minute: '2-digit' })
}
</script>

<!-- 归档页专用扁平任务卡：无投影、低高度，密集列表展示大量归档任务；
     元信息（分组/时间）随宽度自然换行，窄屏时落在标题下方 -->
<template>
  <div class="archive-card">
    <i
      :class="[
        'mdi archive-status',
        task.status === 'done' ? 'mdi-check-circle done' : 'mdi-circle-outline',
      ]"
    />
    <div class="archive-main">
      <div class="archive-line">
        <span class="archive-title" :class="{ struck: task.status === 'done' }">
          {{ task.title }}
        </span>
        <span class="archive-group">
          <i class="mdi mdi-folder-outline"></i>
          {{ groupName }}
        </span>
        <span v-if="task.archived_at" class="archive-time">{{ timeOf(task.archived_at) }}</span>
      </div>
      <!-- 描述单行截断：保持卡片扁平，完整内容恢复后可见 -->
      <div v-if="task.description" class="archive-desc">{{ task.description }}</div>
    </div>
    <v-menu location="bottom right">
      <template #activator="{ props }">
        <v-btn
          v-bind="props"
          icon="mdi-dots-horizontal"
          size="small"
          variant="text"
          class="archive-menu"
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
  </div>
</template>

<style scoped>
.archive-card {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 6px 4px 6px 10px;
  border-radius: 8px;
  /* 无投影扁平条目：低透明度底色在深浅色主题下均可分辨 */
  background: rgba(var(--v-theme-on-surface), 0.045);
  transition: background 0.15s;
}
.archive-card:hover {
  background: rgba(var(--v-theme-on-surface), 0.08);
}
.archive-status {
  flex-shrink: 0;
  margin-top: 2px;
  font-size: 17px;
  color: rgba(var(--v-theme-on-surface), 0.35);
}
.archive-status.done {
  color: rgb(var(--v-theme-primary));
}
.archive-main {
  flex: 1 1 auto;
  min-width: 0;
}
.archive-line {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: 2px 10px;
}
.archive-title {
  flex: 1 1 140px;
  font-size: 14px;
  font-weight: 500;
  line-height: 1.45;
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
  white-space: nowrap;
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
  font-size: 12px;
  line-height: 1.5;
  color: rgba(var(--v-theme-on-surface), 0.55);
  overflow: hidden;
  display: -webkit-box;
  -webkit-line-clamp: 1;
  -webkit-box-orient: vertical;
}
.archive-menu {
  flex-shrink: 0;
}
/* 手机（<600px）：收紧内边距，标题占满首行、元信息自然落到次行 */
@media (max-width: 599.98px) {
  .archive-card {
    gap: 6px;
    padding: 5px 2px 5px 8px;
  }
}
</style>
