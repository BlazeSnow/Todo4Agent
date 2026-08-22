<script setup lang="ts">
import { computed, ref } from 'vue'
import type { Task } from '../types'

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

type SortMode = 'manual' | 'time' | 'title'

const sortMode = ref<SortMode>('manual')
const sortModeLabel = computed(
  () => ({ manual: '手动排序', time: '按截止时间', title: '按标题' })[sortMode.value],
)

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

// ---------- 拖拽重排（仅手动排序模式） ----------

const draggingId = ref<number | null>(null)
const overId = ref<number | null>(null)

function onDragStart(task: Task, e: DragEvent) {
  if (sortMode.value !== 'manual') return
  draggingId.value = task.id
  if (e.dataTransfer) {
    e.dataTransfer.effectAllowed = 'move'
    e.dataTransfer.setData('text/plain', String(task.id))
  }
}

function onDrop(task: Task) {
  const from = draggingId.value
  if (from == null || from === task.id) return
  const list = [...props.tasks]
  const fromIdx = list.findIndex((t) => t.id === from)
  const toIdx = list.findIndex((t) => t.id === task.id)
  if (fromIdx < 0 || toIdx < 0) return
  const [item] = list.splice(fromIdx, 1)
  list.splice(fromIdx < toIdx ? toIdx - 1 : toIdx, 0, item)
  emit('reorder', list.map((t) => t.id))
}

function onDragEnd() {
  draggingId.value = null
  overId.value = null
}

const sortOptions: { value: SortMode; label: string }[] = [
  { value: 'manual', label: '手动排序' },
  { value: 'time', label: '按截止时间' },
  { value: 'title', label: '按标题' },
]
</script>

<template>
  <div>
    <div class="d-flex align-center mb-4">
      <v-icon icon="mdi-folder-outline" class="mr-2" />
      <h2 class="text-h6">{{ groupName ?? '未选择分组' }}</h2>
      <v-spacer />

      <v-menu>
        <template #activator="{ props }">
          <v-btn v-bind="props" variant="text" prepend-icon="mdi-sort-variant">
            排序：{{ sortModeLabel }}
          </v-btn>
        </template>
        <v-list density="compact">
          <v-list-item
            v-for="opt in sortOptions"
            :key="opt.value"
            :title="opt.label"
            :active="sortMode === opt.value"
            @click="sortMode = opt.value"
          />
        </v-list>
      </v-menu>

      <v-btn color="primary" prepend-icon="mdi-plus" class="ml-2" @click="$emit('create')">
        新建任务
      </v-btn>
    </div>

    <v-alert v-if="!groupName" type="info" text="请先在左侧创建分组" />

    <v-progress-linear v-if="loading" indeterminate class="mb-4" />

    <v-card
      v-for="task in displayedTasks"
      :key="task.id"
      class="mb-2"
      variant="outlined"
      :class="{
        'opacity-60': task.status === 'done',
        'drag-target': overId === task.id && draggingId !== task.id,
        'dragging': draggingId === task.id,
      }"
      :draggable="sortMode === 'manual'"
      @dragstart="onDragStart(task, $event)"
      @dragover.prevent="overId = task.id"
      @drop.prevent="onDrop(task)"
      @dragend="onDragEnd"
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

<style scoped>
.dragging {
  opacity: 0.4;
}
.drag-target {
  outline: 2px dashed rgb(var(--v-theme-primary));
  outline-offset: -2px;
}
</style>