<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useDisplay } from 'vuetify'
import GroupSidebar from './components/GroupSidebar.vue'
import GroupDialog from './components/GroupDialog.vue'
import TaskDialog from './components/TaskDialog.vue'
import TaskListView from './components/TaskListView.vue'
import ConfirmDialog from './components/ConfirmDialog.vue'
import SettingsView from './components/SettingsView.vue'
import MCPView from './components/MCPView.vue'
import {
  createGroup,
  createTask,
  deleteGroup,
  deleteTask,
  listGroups,
  listTasks,
  renameGroup,
  reorderTasks,
  updateTask,
} from './api'
import type { Group, Task, TaskInput } from './types'

const groups = ref<Group[]>([])
const tasks = ref<Task[]>([])
const selectedGroupId = ref<number | null>(null)
const drawer = ref(true)
const loadingGroups = ref(false)
const loadingTasks = ref(false)
const snackbar = ref({ show: false, text: '' })

// 侧边栏三档行为：
// - 小屏（<600px）：浮层模式，可收起，不挤压主内容
// - 中屏（600-959px）：常驻占位（v-main 自动偏移，不遮挡），可收起
// - 大屏（>=960px）：常驻，不显示切换按钮，禁止收起
const { width } = useDisplay()
const isSmall = computed(() => width.value < 600)
const isLarge = computed(() => width.value >= 960)

const taskDialog = ref(false)
const editingTask = ref<Task | null>(null)
const groupDialog = ref(false)
const groupDialogMode = ref<'create' | 'rename'>('create')
const groupDialogTarget = ref<Group | null>(null)
const confirmDialog = ref(false)
const confirmAction = ref<{ type: 'group' | 'task'; id: number } | null>(null)
const currentView = ref<'tasks' | 'settings' | 'mcp'>('tasks')

const selectedGroup = computed(
  () => groups.value.find((g) => g.id === selectedGroupId.value) ?? null,
)

function notify(text: string) {
  snackbar.value = { show: true, text }
}

async function loadGroups() {
  loadingGroups.value = true
  try {
    groups.value = await listGroups()
    if (groups.value.length > 0) {
      if (!groups.value.some((g) => g.id === selectedGroupId.value)) {
        selectedGroupId.value = groups.value[0].id
      }
    } else {
      selectedGroupId.value = null
    }
  } catch (e) {
    notify((e as Error).message)
  } finally {
    loadingGroups.value = false
  }
}

async function loadTasks() {
  if (selectedGroupId.value == null) {
    tasks.value = []
    return
  }
  loadingTasks.value = true
  try {
    tasks.value = await listTasks(selectedGroupId.value)
  } catch (e) {
    notify((e as Error).message)
  } finally {
    loadingTasks.value = false
  }
}

watch(selectedGroupId, loadTasks)
onMounted(loadGroups)

// ---------- 分组 ----------

function onSelectGroup(id: number) {
  selectedGroupId.value = id
  currentView.value = 'tasks'
}

function openCreateGroup() {
  groupDialogMode.value = 'create'
  groupDialogTarget.value = null
  groupDialog.value = true
}

function openRenameGroup(group: Group) {
  groupDialogMode.value = 'rename'
  groupDialogTarget.value = group
  groupDialog.value = true
}

async function onGroupDialogSave(name: string) {
  try {
    if (groupDialogMode.value === 'create') {
      await createGroup(name)
      notify(`已创建分组：${name}`)
    } else if (groupDialogTarget.value) {
      await renameGroup(groupDialogTarget.value.id, name)
      notify(`已重命名分组：${name}`)
    }
    await loadGroups()
  } catch (e) {
    notify((e as Error).message)
  }
}

function onDeleteGroup(group: Group) {
  confirmAction.value = { type: 'group', id: group.id }
  confirmDialog.value = true
}

// ---------- 任务 ----------

function openCreateTask() {
  editingTask.value = null
  taskDialog.value = true
}

function openEditTask(task: Task) {
  editingTask.value = task
  taskDialog.value = true
}

async function onTaskDialogSave(input: TaskInput) {
  try {
    if (editingTask.value) {
      await updateTask(editingTask.value.id, {
        group_id: input.group_id,
        title: input.title,
        description: input.description,
        due_at: input.due_at,
      })
      notify('任务已更新')
    } else {
      await createTask(input)
      notify('任务已创建')
    }
    if (selectedGroupId.value !== input.group_id) {
      selectedGroupId.value = input.group_id
    }
    await loadTasks()
  } catch (e) {
    notify((e as Error).message)
  }
}

async function onToggleTask(task: Task) {
  try {
    await updateTask(task.id, {
      status: task.status === 'done' ? 'pending' : 'done',
    })
    await loadTasks()
  } catch (e) {
    notify((e as Error).message)
  }
}

function onDeleteTask(task: Task) {
  confirmAction.value = { type: 'task', id: task.id }
  confirmDialog.value = true
}

/** 拖拽重排后持久化顺序 */
async function onReorderTasks(taskIds: number[]) {
  if (selectedGroupId.value == null) return
  try {
    await reorderTasks(selectedGroupId.value, taskIds)
  } catch (e) {
    notify((e as Error).message)
  }
  await loadTasks()
}

const confirmMessage = computed(() =>
  confirmAction.value?.type === 'group'
    ? '删除后该分组下的任务将一并删除，且不可恢复。确定继续吗？'
    : '删除后不可恢复，确定继续吗？',
)

async function doConfirm() {
  const action = confirmAction.value
  if (!action) return
  confirmDialog.value = false
  try {
    if (action.type === 'group') {
      await deleteGroup(action.id)
      notify('已删除分组')
      await loadGroups()
    } else {
      await deleteTask(action.id)
      notify('已删除任务')
      await loadTasks()
    }
  } catch (e) {
    notify((e as Error).message)
  }
  confirmAction.value = null
}

// ---------- 导出（由设置页触发） ----------

function notifyExported() {
  notify('已导出 JSON')
}
</script>

<template>
  <v-app>
    <v-app-bar app>
      <template #prepend>
        <v-btn
          v-if="!isLarge"
          :icon="drawer ? 'mdi-menu-open' : 'mdi-menu'"
          variant="text"
          aria-label="切换侧边栏"
          @click="drawer = !drawer"
        />
      </template>
      <v-app-bar-title>
        <v-icon icon="mdi-checkbox-marked-circle-outline" class="mr-2" />
        Todo4Agent
        <span class="text-body-2 text-medium-emphasis ml-2">
          为 Agent 设计的 MCP 任务清单
        </span>
      </v-app-bar-title>
      <v-btn variant="text" prepend-icon="mdi-refresh" @click="loadGroups">刷新</v-btn>
    </v-app-bar>

    <v-navigation-drawer
      :model-value="isLarge ? true : drawer"
      app
      width="280"
      :temporary="isSmall"
      :mobile-breakpoint="600"
      @update:model-value="(v) => (drawer = v)"
    >
      <GroupSidebar
        :groups="groups"
        :selected-id="selectedGroupId"
        :loading="loadingGroups"
        :active-view="currentView"
        @select="onSelectGroup"
        @create="openCreateGroup"
        @rename="openRenameGroup"
        @delete="onDeleteGroup"
        @mcp="currentView = 'mcp'"
        @settings="currentView = 'settings'"
      />
    </v-navigation-drawer>

    <v-main>
      <v-container fluid class="pa-4">
        <TaskListView
          v-if="currentView === 'tasks'"
          :tasks="tasks"
          :loading="loadingTasks"
          :group-name="selectedGroup?.name ?? null"
          @create="openCreateTask"
          @edit="openEditTask"
          @toggle="onToggleTask"
          @remove="onDeleteTask"
          @reorder="onReorderTasks"
        />
        <SettingsView
          v-else-if="currentView === 'settings'"
          @exported="notifyExported"
          @error="notify"
        />
        <MCPView v-else @notify="notify" />
      </v-container>
    </v-main>

    <TaskDialog
      v-model="taskDialog"
      :task="editingTask"
      :groups="groups"
      :default-group-id="selectedGroupId"
      @save="onTaskDialogSave"
    />
    <GroupDialog
      v-model="groupDialog"
      :mode="groupDialogMode"
      :group="groupDialogTarget"
      @save="onGroupDialogSave"
    />

    <ConfirmDialog v-model="confirmDialog" :message="confirmMessage" @confirm="doConfirm" />

    <v-snackbar v-model="snackbar.show" :timeout="3000" location="bottom">
      {{ snackbar.text }}
    </v-snackbar>
  </v-app>
</template>