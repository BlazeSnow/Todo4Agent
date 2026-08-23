<script setup lang="ts">
import { computed, ref } from 'vue'
import type { Task } from '../types'
import ContextMenu, { type ContextMenuItem } from './ContextMenu.vue'

const props = defineProps<{
  tasks: Task[]
  loading: boolean
  /** 当前分组名（未选择时为 null） */
  groupName: string | null
}>()

const emit = defineEmits<{
  (e: 'create'): void
  (e: 'edit', task: Task): void
  (e: 'toggle', task: Task): void
  (e: 'remove', task: Task): void
  (e: 'reorder', taskIds: number[]): void
}>()

// ---------- 排序 ----------

type SortMode = 'time' | 'title'

/** null 表示默认顺序（后端返回的原始顺序） */
const sortMode = ref<SortMode | null>(null)
const sortModeLabel = computed(() => {
  if (sortMode.value === null) return '默认顺序'
  return sortMode.value === 'time' ? '按截止时间' : '按标题'
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
    list.sort((a, b) => a.title.localeCompare(b.title, 'zh-Hans-CN'))
  }
  return list
})

/** 点击已选中的排序项时恢复默认顺序 */
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
        label: task.status === 'done' ? '标记未完成' : '标记完成',
        icon: 'mdi-check',
        action: () => emit('toggle', task),
      },
      // 非默认排序下移动无意义，与「更多操作」菜单保持一致
      ...(sortMode.value === null
        ? [
            {
              label: '上移',
              icon: 'mdi-arrow-up',
              disabled: !canMove(task, -1),
              action: () => moveTask(task, -1),
            },
            {
              label: '下移',
              icon: 'mdi-arrow-down',
              disabled: !canMove(task, 1),
              action: () => moveTask(task, 1),
            },
          ]
        : []),
      { divider: true },
      { label: '编辑', icon: 'mdi-pencil', action: () => emit('edit', task) },
      {
        label: '删除',
        icon: 'mdi-delete',
        color: 'error',
        action: () => emit('remove', task),
      },
    ],
  }
}

const sortOptions: { value: SortMode | null; label: string }[] = [
  { value: null, label: '默认顺序' },
  { value: 'time', label: '按截止时间' },
  { value: 'title', label: '按标题' },
]

// ---------- 展示辅助 ----------

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
    <div class="list-header">
      <h2 class="group-title">{{ groupName ?? '未选择分组' }}</h2>
      <div class="header-actions">
        <v-menu>
          <template #activator="{ props }">
            <v-btn v-bind="props" variant="text" prepend-icon="mdi-sort-variant">
              排序：{{ sortModeLabel }}
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
        <v-btn color="primary" prepend-icon="mdi-plus" @click="$emit('create')">
          新建任务
        </v-btn>
      </div>
    </div>

    <div v-if="!groupName" class="empty-tip">请先在左侧创建分组</div>

    <div v-if="loading" class="list-loading" />

    <!-- 任务卡片（原生结构，样式与回收站共享，见 styles/task-card.css） -->
    <div class="task-item" v-for="task in displayedTasks" :key="task.id" @contextmenu.stop="openTaskCtx(task, $event)">
      <input
        type="checkbox"
        class="task-check task-lead"
        :checked="task.status === 'done'"
        :aria-label="`完成：${task.title}`"
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
              :aria-label="`更多操作：${task.title}`"
            />
          </template>
          <v-list density="compact">
            <v-list-item
              prepend-icon="mdi-arrow-up"
              title="上移"
              :disabled="!canMove(task, -1)"
              @click="moveTask(task, -1)"
            />
            <v-list-item
              prepend-icon="mdi-arrow-down"
              title="下移"
              :disabled="!canMove(task, 1)"
              @click="moveTask(task, 1)"
            />
            <v-divider />
            <v-list-item
              prepend-icon="mdi-pencil"
              title="编辑"
              @click="$emit('edit', task)"
            />
            <v-list-item
              prepend-icon="mdi-delete"
              title="删除"
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
      <div class="empty-title">暂无任务</div>
      <div class="empty-text">点击右上角「新建任务」，或让 Agent 通过 MCP 添加</div>
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
.group-title {
  font-size: 18px;
  font-weight: 600;
  margin: 0;
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