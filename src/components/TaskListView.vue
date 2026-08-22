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
function toggleSort(mode: SortMode) {
  sortMode.value = sortMode.value === mode ? null : mode
}

const sortOptions: { value: SortMode; label: string }[] = [
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
            @click="toggleSort(opt.value)"
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