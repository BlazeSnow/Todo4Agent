<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { Task } from '../types'
import { NO_GROUP_NAME } from '../types'
import { dateLocale, sortLocale } from '../i18n'
import ContextMenu, { type ContextMenuItem } from './ContextMenu.vue'

const props = defineProps<{
  tasks: Task[]
  loading: boolean
  /** 当前分组名（未选择时为 null） */
  groupName: string | null
  /** 当前分组描述（未设置或未选择时为 null） */
  groupDescription: string | null
}>()

const emit = defineEmits<{
  (e: 'create'): void
  (e: 'edit', task: Task): void
  (e: 'archive', task: Task): void
  (e: 'toggle', task: Task): void
  (e: 'remove', task: Task): void
  (e: 'reorder', taskIds: number[]): void
}>()

const { t } = useI18n()

/** 标题区显示的分组名：系统分组「无分组」按界面语言显示 */
const headingName = computed(() =>
  props.groupName === NO_GROUP_NAME ? t('groups.noGroup') : props.groupName,
)

// ---------- 排序 ----------

type SortMode = 'time' | 'title'

/** null 表示默认顺序（后端返回的原始顺序） */
const sortMode = ref<SortMode | null>(null)
const sortModeLabel = computed(() => {
  if (sortMode.value === null) return t('taskList.sortDefault')
  return sortMode.value === 'time' ? t('taskList.sortTime') : t('taskList.sortTitle')
})

/** 按当前排序模式展示的任务列表（不修改原始数组） */
const displayedTasks = computed<Task[]>(() => {
  const list = [...props.tasks]
  if (sortMode.value === 'time') {
    // 按任务设置的截止时间排序：无截止时间的排最后，到期早的在前
    list.sort((a, b) => {
      if (a.due_at == null && b.due_at == null) return 0
      if (a.due_at == null) return 1
      if (b.due_at == null) return -1
      return a.due_at.localeCompare(b.due_at)
    })
  } else if (sortMode.value === 'title') {
    list.sort((a, b) => a.title.localeCompare(b.title, sortLocale.value))
  }
  return list
})

const sortOptions = computed(() => [
  { value: null, label: t('taskList.sortDefault') },
  { value: 'time' as SortMode, label: t('taskList.sortTime') },
  { value: 'title' as SortMode, label: t('taskList.sortTitle') },
])

/** 选择排序方式；null 为默认顺序 */
function toggleSort(mode: SortMode | null) {
  sortMode.value = mode
}

// ---------- 上移 / 下移 ----------

/** 仅默认顺序下可上移/下移（其他排序模式下顺序由排序决定，移动无意义） */
function canMove(task: Task, dir: -1 | 1): boolean {
  if (sortMode.value !== null) return false
  const idx = displayedTasks.value.findIndex((t) => t.id === task.id)
  if (idx < 0) return false
  const target = idx + dir
  return target >= 0 && target < displayedTasks.value.length
}

function moveTask(task: Task, dir: -1 | 1) {
  if (!canMove(task, dir)) return
  const list = [...displayedTasks.value]
  const idx = list.findIndex((t) => t.id === task.id)
  const target = idx + dir
  const tmp = list[idx]
  list[idx] = list[target]
  list[target] = tmp
  emit('reorder', list.map((t) => t.id))
}

// ---------- 右键菜单 ----------

const taskCtx = ref<{ x: number; y: number; items: ContextMenuItem[] } | null>(null)

function openTaskCtx(task: Task, e: MouseEvent) {
  e.preventDefault()
  taskCtx.value = {
    x: e.clientX,
    y: e.clientY,
    items: [
      {
        label: task.status === 'done' ? t('taskList.markUndone') : t('taskList.markDone'),
        icon: 'mdi-check',
        action: () => emit('toggle', task),
      },
      // 非默认排序下移动无意义，与「更多操作」菜单保持一致
      ...(sortMode.value === null
        ? [
            {
              label: t('common.moveUp'),
              icon: 'mdi-arrow-up',
              disabled: !canMove(task, -1),
              action: () => moveTask(task, -1),
            },
            {
              label: t('common.moveDown'),
              icon: 'mdi-arrow-down',
              disabled: !canMove(task, 1),
              action: () => moveTask(task, 1),
            },
          ]
        : []),
      { divider: true },
      { label: t('common.edit'), icon: 'mdi-pencil', action: () => emit('edit', task) },
      {
        label: t('taskList.archive'),
        icon: 'mdi-archive-arrow-down-outline',
        action: () => emit('archive', task),
      },
      {
        label: t('common.delete'),
        icon: 'mdi-delete',
        color: 'error',
        action: () => emit('remove', task),
      },
    ],
  }
}

// ---------- 双击编辑 ----------

/** 双击卡片打开编辑；起始于勾选框或操作按钮的双击不触发（保留其原生行为） */
function onDblClick(task: Task, e: MouseEvent) {
  const el = e.target as HTMLElement
  if (el.closest('.task-check, .task-actions')) return
  emit('edit', task)
}

// ---------- 展示辅助 ----------

function formatDue(iso: string): string {
  const d = new Date(iso)
  if (isNaN(d.getTime())) return iso
  return d.toLocaleString(dateLocale.value, { dateStyle: 'medium', timeStyle: 'short' })
}

function overdue(task: Task): boolean {
  if (!task.due_at || task.status === 'done') return false
  const d = new Date(task.due_at)
  return !isNaN(d.getTime()) && d.getTime() < Date.now()
}
</script>

<template>
  <div>
    <div class="list-header">
      <div class="group-heading">
        <h2 class="group-title">{{ headingName ?? t('taskList.noGroupSelected') }}</h2>
        <div v-if="groupDescription" class="group-desc">{{ groupDescription }}</div>
      </div>
      <div class="header-actions">
        <v-menu>
          <template #activator="{ props }">
            <!-- 小屏（手机）仅显示图标，≥sm 断点恢复文字 -->
            <v-btn
              v-bind="props"
              variant="text"
              prepend-icon="mdi-sort-variant"
              :aria-label="t('taskList.sort', { mode: sortModeLabel })"
            >
              <span class="d-none d-sm-inline">
                {{ t('taskList.sort', { mode: sortModeLabel }) }}
              </span>
            </v-btn>
          </template>
          <v-list density="compact">
            <v-list-item
              v-for="opt in sortOptions"
              :key="opt.value ?? 'default'"
              :title="opt.label"
              :active="sortMode === opt.value"
              @click="toggleSort(opt.value)"
            />
          </v-list>
        </v-menu>
        <v-btn color="primary" prepend-icon="mdi-plus" :aria-label="t('taskList.newTask')" @click="$emit('create')">
          <span class="d-none d-sm-inline">{{ t('taskList.newTask') }}</span>
        </v-btn>
      </div>
    </div>

    <div v-if="!groupName" class="empty-tip">{{ t('taskList.createGroupFirst') }}</div>

    <div v-if="loading" class="list-loading" />

    <!-- 任务卡片（原生结构，样式与回收站共享，见 styles/task-card.css） -->
    <div
      class="task-item"
      v-for="task in displayedTasks"
      :key="task.id"
      :title="t('taskList.dblclickToEdit', { title: task.title })"
      @contextmenu.stop="openTaskCtx(task, $event)"
      @dblclick="onDblClick(task, $event)"
    >
      <input
        type="checkbox"
        class="task-check task-lead"
        :checked="task.status === 'done'"
        :aria-label="t('taskList.completeAria', { title: task.title })"
        @change="$emit('toggle', task)"
      />
      <div class="task-main">
        <div class="task-title" :class="{ struck: task.status === 'done' }">
          {{ task.title }}
        </div>
        <div v-if="task.description" class="task-desc">{{ task.description }}</div>
        <div v-if="task.due_at" class="task-pills">
          <span class="task-pill" :class="{ overdue: overdue(task) }">
            <i class="mdi mdi-calendar"></i>
            {{ formatDue(task.due_at) }}
          </span>
        </div>
      </div>
      <div class="task-actions">
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
              prepend-icon="mdi-arrow-up"
              :title="t('common.moveUp')"
              :disabled="!canMove(task, -1)"
              @click="moveTask(task, -1)"
            />
            <v-list-item
              prepend-icon="mdi-arrow-down"
              :title="t('common.moveDown')"
              :disabled="!canMove(task, 1)"
              @click="moveTask(task, 1)"
            />
            <v-divider />
            <v-list-item
              prepend-icon="mdi-pencil"
              :title="t('common.edit')"
              @click="$emit('edit', task)"
            />
            <v-list-item
              prepend-icon="mdi-archive-arrow-down-outline"
              :title="t('taskList.archive')"
              :subtitle="t('taskList.archiveSubtitle')"
              @click="$emit('archive', task)"
            />
            <v-list-item
              prepend-icon="mdi-delete"
              :title="t('common.delete')"
              color="error"
              @click="$emit('remove', task)"
            />
          </v-list>
        </v-menu>
      </div>
    </div>

    <div
      v-if="!loading && groupName && tasks.length === 0"
      class="empty-state"
    >
      <i class="mdi mdi-inbox-outline"></i>
      <div class="empty-title">{{ t('taskList.empty') }}</div>
      <div class="empty-text">{{ t('taskList.emptyHint') }}</div>
    </div>

    <ContextMenu
      v-if="taskCtx"
      :items="taskCtx.items"
      :x="taskCtx.x"
      :y="taskCtx.y"
      @close="taskCtx = null"
    />
  </div>
</template>

<style src="../styles/task-card.css"></style>

<style scoped>
.list-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 16px;
}
.group-heading {
  min-width: 0;
}
.group-title {
  font-size: 18px;
  font-weight: 600;
  margin: 0;
}
.group-desc {
  font-size: 13px;
  color: rgba(var(--v-theme-on-surface), 0.6);
  white-space: pre-wrap;
  word-break: break-word;
  margin-top: 2px;
}
.header-actions {
  margin-left: auto;
  display: flex;
  align-items: center;
  gap: 8px;
}

.empty-tip {
  padding: 12px 16px;
  border-radius: 8px;
  background: rgba(var(--v-theme-info), 0.12);
  color: rgb(var(--v-theme-info));
  font-size: 14px;
  margin-bottom: 12px;
}

.list-loading {
  height: 4px;
  border-radius: 2px;
  margin-bottom: 16px;
  background: linear-gradient(
    90deg,
    transparent,
    rgba(var(--v-theme-primary), 0.4),
    transparent
  );
  background-size: 200% 100%;
  animation: loading-slide 1.2s infinite;
}
@keyframes loading-slide {
  from {
    background-position: 200% 0;
  }
  to {
    background-position: -200% 0;
  }
}

.task-check {
  width: 18px;
  height: 18px;
  cursor: pointer;
  accent-color: #00a862;
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  padding: 48px 0;
  color: rgba(var(--v-theme-on-surface), 0.4);
}
.empty-state .mdi {
  font-size: 48px;
  margin-bottom: 8px;
}
.empty-title {
  font-size: 15px;
  font-weight: 500;
  color: rgba(var(--v-theme-on-surface), 0.6);
}
.empty-text {
  font-size: 13px;
}
</style>