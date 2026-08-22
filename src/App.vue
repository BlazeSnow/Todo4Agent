<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useDisplay } from 'vuetify'
import GroupSidebar from './components/GroupSidebar.vue'
import GroupDialog from './components/GroupDialog.vue'
import TaskDialog from './components/TaskDialog.vue'
import {
  createGroup,
  createTask,
  deleteGroup,
  deleteTask,
  downloadExport,
  exportDoc,
  listGroups,
  listTasks,
  renameGroup,
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

// 窄屏（手机/小窗口）时侧边栏自动切换为浮层模式，不挤压主内容
const { mobile } = useDisplay()

const taskDialog = ref(false)
const editingTask = ref<Task | null>(null)
const groupDialog = ref(false)
const groupDialogMode = ref<'create' | 'rename'>('create')
const groupDialogTarget = ref<Group | null>(null)
const confirmDialog = ref(false)
const confirmAction = ref<{ type: 'group' | 'task'; id: number } | null>(null)
const mcpDialog = ref(false)

// MCP 工具清单（与后端 mcp.rs 保持一致）
const mcpTools = [
  'group_list / group_create / group_rename',
  'task_list / task_create / task_update',
  'task_complete / task_delete / task_export',
]

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

async function doConfirm() {
  const action = confirmAction.value
  if (!action) return
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

// ---------- 导出 ----------

async function onExport() {
  try {
    const doc = await exportDoc()
    downloadExport(doc)
    notify('已导出 JSON')
  } catch (e) {
    notify((e as Error).message)
  }
}

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

async function copyMcpCommand() {
  try {
    await navigator.clipboard.writeText('todo4agent mcp')
    notify('已复制命令：todo4agent mcp')
  } catch {
    notify('复制失败，请手动复制')
  }
}
</script>

<template>
  <v-app>
    <v-app-bar app>
      <template #prepend>
        <v-btn
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
      <v-btn variant="text" prepend-icon="mdi-export-variant" @click="onExport">
        导出 JSON
      </v-btn>
      <v-btn variant="text" prepend-icon="mdi-refresh" @click="loadGroups">刷新</v-btn>
    </v-app-bar>

    <v-navigation-drawer v-model="drawer" app width="280" :temporary="mobile">
      <GroupSidebar
        :groups="groups"
        :selected-id="selectedGroupId"
        :loading="loadingGroups"
        @select="onSelectGroup"
        @create="openCreateGroup"
        @rename="openRenameGroup"
        @delete="onDeleteGroup"
        @mcp="mcpDialog = true"
      />
    </v-navigation-drawer>

    <v-main>
      <v-container fluid class="pa-4">
        <div class="d-flex align-center mb-4">
          <v-icon icon="mdi-folder-outline" class="mr-2" />
          <h2 class="text-h6">{{ selectedGroup?.name ?? '未选择分组' }}</h2>
          <v-spacer />
          <v-btn color="primary" prepend-icon="mdi-plus" @click="openCreateTask">
            新建任务
          </v-btn>
        </div>

        <v-alert v-if="!selectedGroup" type="info" text="请先在左侧创建分组" />

        <v-progress-linear v-if="loadingTasks" indeterminate class="mb-4" />

        <v-card
          v-for="task in tasks"
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
                @update:model-value="onToggleTask(task)"
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
              <v-chip
                size="small"
                variant="tonal"
                :color="overdue(task) ? 'error' : 'default'"
              >
                <v-icon start icon="mdi-calendar" size="small" />
                {{ formatDue(task.due_at) }}
              </v-chip>
            </v-list-item-subtitle>
            <template #append>
              <v-btn
                icon="mdi-pencil"
                size="small"
                variant="text"
                @click="openEditTask(task)"
              />
              <v-btn
                icon="mdi-delete"
                size="small"
                variant="text"
                color="error"
                @click="onDeleteTask(task)"
              />
            </template>
          </v-list-item>
        </v-card>

        <v-empty
          v-if="!loadingTasks && selectedGroup && tasks.length === 0"
          icon="mdi-inbox-outline"
          title="暂无任务"
          text="点击右上角「新建任务」，或让 Agent 通过 MCP 添加"
        />
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

    <v-dialog v-model="confirmDialog" max-width="420">
      <v-card>
        <v-card-title>确认删除</v-card-title>
        <v-card-text>
          {{
            confirmAction?.type === 'group'
              ? '删除后该分组下的任务将一并删除，且不可恢复。确定继续吗？'
              : '删除后不可恢复，确定继续吗？'
          }}
        </v-card-text>
        <v-card-actions>
          <v-spacer />
          <v-btn variant="text" @click="confirmDialog = false">取消</v-btn>
          <v-btn
            color="error"
            @click="
              confirmDialog = false;
              doConfirm()
            "
          >
            删除
          </v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>

    <v-dialog v-model="mcpDialog" max-width="560">
      <v-card>
        <v-card-title>
          <v-icon icon="mdi-connection" class="mr-2" />
          Agent 接入（MCP）
        </v-card-title>
        <v-card-text>
          <p class="mb-2">
            本软件通过 MCP（Model Context Protocol，stdio 传输）向 Agent 暴露任务清单能力，
            Agent 以子进程方式启动并连接，与桌面端共用同一个数据库。
          </p>

          <div class="d-flex align-center">
            <v-chip
              class="font-mono mr-2"
              variant="outlined"
              label
              data-testid="mcp-command"
            >
              todo4agent mcp
            </v-chip>
            <v-btn
              size="small"
              variant="tonal"
              prepend-icon="mdi-content-copy"
              @click="copyMcpCommand"
            >
              复制命令
            </v-btn>
          </div>

          <v-divider class="my-3" />

          <div class="text-subtitle-2 mb-1">Agent 客户端配置示例：</div>
          <pre class="bg-grey-lighten-3 pa-3 rounded"><code>{
  "mcpServers": {
    "todo4agent": {
      "command": "todo4agent",
      "args": ["mcp"]
    }
  }
}</code></pre>

          <v-divider class="my-3" />

          <div class="text-subtitle-2 mb-1">可用工具：</div>
          <v-list density="compact">
            <v-list-item v-for="t in mcpTools" :key="t">
              <template #prepend>
                <v-icon icon="mdi-wrench" size="small" class="mr-2" />
              </template>
              <v-list-item-title class="font-mono text-body-2">{{ t }}</v-list-item-title>
            </v-list-item>
          </v-list>
        </v-card-text>
        <v-card-actions>
          <v-spacer />
          <v-btn variant="text" @click="mcpDialog = false">关闭</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>

    <v-snackbar v-model="snackbar.show" :timeout="3000" location="bottom">
      {{ snackbar.text }}
    </v-snackbar>
  </v-app>
</template>