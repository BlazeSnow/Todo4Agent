<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useDisplay } from 'vuetify'
import GroupSidebar from './components/GroupSidebar.vue'
import ContextMenu, { type ContextMenuItem } from './components/ContextMenu.vue'
import GroupDialog from './components/GroupDialog.vue'
import TaskDialog from './components/TaskDialog.vue'
import TaskListView from './components/TaskListView.vue'
import ConfirmDialog from './components/ConfirmDialog.vue'
import SettingsView from './components/SettingsView.vue'
import MCPView from './components/MCPView.vue'
import PromptView from './components/PromptView.vue'
import TrashView from './components/TrashView.vue'
import ArchiveView from './components/ArchiveView.vue'
import LoginView from './components/LoginView.vue'
import LocaleSwitch from './components/LocaleSwitch.vue'
import {
  authLogout,
  authStatus,
  createGroup,
  createTask,
  deleteGroup,
  deleteTask,
  emptyTrash,
  archiveTask,
  listArchive,
  listGroups,
  listTasks,
  listTrash,
  purgeGroup,
  purgeTask,
  unarchiveTask,
  updateGroup,
  reorderGroups,
  reorderTasks,
  restoreGroup,
  restoreTask,
  setGroupLocked,
  setToken,
  updateTask,
} from './api'
import type { Group, Task, TaskInput } from './types'
import type PromptViewType from './components/PromptView.vue'

const { t } = useI18n()

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
type TrashAction = 'group' | 'task' | 'purgeGroup' | 'purgeTask' | 'emptyTrash' | 'archivedTask'
const confirmAction = ref<{ type: TrashAction; id?: number } | null>(null)
const currentView = ref<'tasks' | 'settings' | 'mcp' | 'prompt' | 'archive' | 'trash'>('tasks')

// ---------- 路径路由（保留当前页面，刷新后不回默认界面） ----------

type ViewName = (typeof currentView.value)

/** 从路径解析视图与分组：/group/{id}、/archive、/trash、/mcp、/prompt、/settings */
function parseRoute(): { view: ViewName | null; groupId: number | null } {
  const seg = location.pathname.replace(/\/+$/, '').split('/').filter(Boolean)
  if (seg.length === 1) {
    const v = seg[0]
    if (v === 'archive' || v === 'trash' || v === 'mcp' || v === 'prompt' || v === 'settings') {
      return { view: v, groupId: null }
    }
  } else if (seg.length === 2 && seg[0] === 'group') {
    const n = Number(seg[1])
    if (Number.isInteger(n) && n > 0) return { view: 'tasks', groupId: n }
  }
  return { view: null, groupId: null }
}

/** 当前状态对应的路径 */
function routePath(): string {
  if (currentView.value !== 'tasks') return `/${currentView.value}`
  return selectedGroupId.value != null ? `/group/${selectedGroupId.value}` : '/'
}

/** 页面生命周期内的首次路径同步：replaceState 避免首个默认分组压入多余历史记录 */
let routeSynced = false

/** 状态变化 → 地址栏（路径相同不操作） */
function syncRoute() {
  const p = routePath()
  if (location.pathname === p) return
  if (routeSynced) history.pushState({}, '', p)
  else {
    history.replaceState({}, '', p)
    routeSynced = true
  }
}

/** 地址栏 → 状态（初始加载 / 登录后 / 浏览器前进后退） */
function applyRoute() {
  const r = parseRoute()
  if (r.view) currentView.value = r.view
  if (r.groupId != null) selectedGroupId.value = r.groupId
}

function onPopState() {
  applyRoute()
}

watch([currentView, selectedGroupId], syncRoute)

// ---------- 认证门控 ----------

type AuthState = 'loading' | 'guest' | 'ready'
const authState = ref<AuthState>('loading')
const currentUser = ref<string | null>(null)

/** 校验当前会话：需有效 token，否则进入登录页 */
async function initAuth() {
  try {
    const s = await authStatus()
    if (s.user_id != null) {
      currentUser.value = s.username
      authState.value = 'ready'
    } else {
      authState.value = 'guest'
    }
  } catch {
    authState.value = 'guest'
  }
}

function onLoggedIn(username: string) {
  currentUser.value = username
  authState.value = 'ready'
  applyRoute()
  loadGroups()
}

async function onLogout() {
  try {
    await authLogout()
  } catch {
    // 忽略登出接口异常，本地 token 仍清除
  }
  setToken(null)
  currentUser.value = null
  authState.value = 'guest'
}

// 回收站
const trashGroups = ref<Group[]>([])
const trashTasks = ref<Task[]>([])

async function loadTrash() {
  try {
    const data = await listTrash()
    trashGroups.value = data.groups
    trashTasks.value = data.tasks
  } catch (e) {
    notify((e as Error).message)
  }
}

watch(currentView, (v) => {
  if (v === 'archive') loadArchive()
  if (v === 'trash') loadTrash()
})

// 归档
const archiveTasks = ref<Task[]>([])

async function loadArchive() {
  try {
    archiveTasks.value = await listArchive()
  } catch (e) {
    notify((e as Error).message)
  }
}

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

/** 手动刷新：重载分组与当前视图数据（MCP 等外部修改后同步界面） */
const promptView = ref<InstanceType<typeof PromptViewType> | null>(null)

async function refresh() {
  await loadGroups()
  if (selectedGroupId.value != null) await loadTasks()
  if (currentView.value === 'archive') await loadArchive()
  if (currentView.value === 'trash') await loadTrash()
  // 提示词可被 Agent 经 MCP 修改，刷新时一并重载（覆盖本地未保存编辑）
  if (currentView.value === 'prompt') await promptView.value?.reload()
}

const authReady = computed(() => authState.value !== 'loading')

/** 淡出并移除 index.html 的开屏动画（应用首屏已就绪） */
function dismissSplash() {
  const el = document.getElementById('splash')
  if (!el) return
  el.classList.add('splash-out')
  window.setTimeout(() => el.remove(), 500)
}

onMounted(async () => {
  window.addEventListener('popstate', onPopState)
  await initAuth()
  // 首屏（登录页或主界面）渲染完成后淡出开屏，分组数据在其后继续加载
  await nextTick()
  dismissSplash()
  if (authState.value !== 'guest') {
    // 先按路径恢复视图与分组，再加载分组列表校验（失效分组回退到第一个）
    applyRoute()
    await loadGroups()
  }
})

onBeforeUnmount(() => {
  window.removeEventListener('popstate', onPopState)
})

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

async function onGroupDialogSave(name: string, description: string) {
  try {
    if (groupDialogMode.value === 'create') {
      await createGroup(name, description)
      notify(t('app.groupCreated', { name }))
    } else if (groupDialogTarget.value) {
      await updateGroup(groupDialogTarget.value.id, { name, description })
      notify(t('app.groupSaved', { name }))
    }
    await loadGroups()
  } catch (e) {
    notify((e as Error).message)
  }
}

/** 切换清单锁定：锁定后该清单 Agent 无法通过 MCP 编辑，界面编辑不受影响 */
async function onToggleGroupLock(group: Group) {
  try {
    const g = await setGroupLocked(group.id, !group.locked)
    notify(g.locked ? t('app.groupLocked', { name: g.name }) : t('app.groupUnlocked', { name: g.name }))
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

/** 归档任务：从清单移入归档（时间线保留，可恢复） */
async function onArchiveTask(task: Task) {
  try {
    await archiveTask(task.id)
    notify(t('app.taskArchived', { title: task.title }))
    await loadTasks()
  } catch (e) {
    notify((e as Error).message)
  }
}

/** 取消归档：任务回到原清单 */
async function onUnarchiveTask(task: Task) {
  try {
    await unarchiveTask(task.id)
    notify(t('app.taskUnarchived', { title: task.title }))
    await loadArchive()
    // 任务回到清单：当前分组列表可能仍是旧数据，一并刷新
    await loadTasks()
  } catch (e) {
    notify((e as Error).message)
  }
}

function onRemoveArchivedTask(task: Task) {
  confirmAction.value = { type: 'archivedTask', id: task.id }
  confirmDialog.value = true
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
      notify(t('app.taskUpdated'))
    } else {
      await createTask(input)
      notify(t('app.taskCreated'))
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

/** 上移/下移后持久化新顺序 */
async function onReorderTasks(taskIds: number[]) {
  if (selectedGroupId.value == null) return
  try {
    await reorderTasks(selectedGroupId.value, taskIds)
  } catch (e) {
    notify((e as Error).message)
  }
  await loadTasks()
}

/** 分组上移/下移后持久化新顺序 */
async function onReorderGroups(groupIds: number[]) {
  try {
    await reorderGroups(groupIds)
  } catch (e) {
    notify((e as Error).message)
  }
  await loadGroups()
}

const confirmMessage = computed(() => {
  switch (confirmAction.value?.type) {
    case 'group':
      return t('confirm.deleteGroup')
    case 'task':
      return t('confirm.deleteTask')
    case 'purgeGroup':
      return t('confirm.purgeGroup')
    case 'purgeTask':
      return t('confirm.purgeTask')
    case 'emptyTrash':
      return t('confirm.emptyTrash')
    case 'archivedTask':
      return t('confirm.archivedTask')
    default:
      return ''
  }
})

async function doConfirm() {
  const action = confirmAction.value
  if (!action) return
  confirmDialog.value = false
  try {
    switch (action.type) {
      case 'group':
        await deleteGroup(action.id!)
        notify(t('app.movedToTrash'))
        await loadGroups()
        break
      case 'task':
        await deleteTask(action.id!)
        notify(t('app.movedToTrash'))
        await loadTasks()
        break
      case 'purgeGroup':
        await purgeGroup(action.id!)
        notify(t('app.groupPurged'))
        await loadTrash()
        break
      case 'purgeTask':
        await purgeTask(action.id!)
        notify(t('app.taskPurged'))
        await loadTrash()
        break
      case 'emptyTrash':
        await emptyTrash()
        notify(t('app.trashEmptied'))
        await loadTrash()
        break
      case 'archivedTask':
        await deleteTask(action.id!)
        notify(t('app.movedToTrash'))
        await loadArchive()
        break
    }
  } catch (e) {
    notify((e as Error).message)
  }
  confirmAction.value = null
}

async function onRestoreTrash(kind: 'group' | 'task', id: number) {
  try {
    if (kind === 'task') {
      await restoreTask(id)
      notify(t('app.taskRestored'))
      await loadTasks()
    } else {
      const r = await restoreGroup(id)
      notify(r.renamed_to ? t('app.groupRestoredRenamed', { name: r.renamed_to }) : t('app.groupRestored'))
      await loadGroups()
    }
    await loadTrash()
  } catch (e) {
    notify((e as Error).message)
  }
}

function onPurgeTrash(kind: 'group' | 'task', id: number) {
  confirmAction.value = { type: kind === 'task' ? 'purgeTask' : 'purgeGroup', id }
  confirmDialog.value = true
}

function onEmptyTrash() {
  confirmAction.value = { type: 'emptyTrash' }
  confirmDialog.value = true
}

// ---------- 导出（由设置页触发） ----------

function notifyExported() {
  notify(t('app.exported'))
}

/** 导入完成后刷新各视图数据 */
async function onImported() {
  await Promise.all([loadGroups(), loadTrash()])
  if (selectedGroupId.value != null) await loadTasks()
}

// ---------- 全局右键菜单（接管浏览器默认菜单） ----------

const globalCtx = ref<{ x: number; y: number; items: ContextMenuItem[] } | null>(null)

function onGlobalContextMenu(e: MouseEvent) {
  const target = e.target as HTMLElement
  // 输入类元素保留 WebView2 原生的复制/粘贴菜单
  if (target.closest('input, textarea, [contenteditable="true"]')) return
  e.preventDefault()
  globalCtx.value = {
    x: e.clientX,
    y: e.clientY,
    items: [
      {
        label: t('app.newTask'),
        icon: 'mdi-plus',
        disabled: currentView.value !== 'tasks' || selectedGroupId.value == null,
        action: openCreateTask,
      },
      { label: t('common.refresh'), icon: 'mdi-refresh', action: () => refresh() },
    ],
  }
}

onMounted(() => window.addEventListener('contextmenu', onGlobalContextMenu))
onBeforeUnmount(() => window.removeEventListener('contextmenu', onGlobalContextMenu))
</script>

<template>
  <v-app>
    <v-progress-linear v-if="authState === 'loading'" indeterminate />

    <LoginView
      v-else-if="authState === 'guest'"
      @logged-in="onLoggedIn"
      @error="notify"
    />

    <template v-else>
    <v-app-bar app>
      <template #prepend>
        <v-btn
          v-if="!isLarge"
          :icon="drawer ? 'mdi-menu-open' : 'mdi-menu'"
          variant="text"
          :aria-label="t('app.toggleSidebar')"
          @click="drawer = !drawer"
        />
      </template>
      <v-app-bar-title>
        <img src="/favicon.ico" alt="Todo4Agent" class="app-logo" />
        <!-- 小屏（手机）仅显示 Logo，≥sm 断点恢复应用名与副标题 -->
        <span class="d-none d-sm-inline">Todo4Agent</span>
        <span class="text-body-2 text-medium-emphasis ml-2 d-none d-sm-inline">
          {{ t('app.tagline') }}
        </span>
      </v-app-bar-title>
      <LocaleSwitch />
      <!-- 小屏（手机）切换为 Vuetify 原生图标按钮（正方形、图标居中）；
           注意 icon 属性仅在无默认插槽时才渲染图标，文字须走 text 属性 -->
      <v-btn
        variant="text"
        :icon="isSmall ? 'mdi-refresh' : undefined"
        :prepend-icon="isSmall ? undefined : 'mdi-refresh'"
        :text="isSmall ? undefined : t('common.refresh')"
        :aria-label="t('common.refresh')"
        @click="refresh"
      />
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
        @toggle-lock="onToggleGroupLock"
        @mcp="currentView = 'mcp'"
        @prompt="currentView = 'prompt'"
        @settings="currentView = 'settings'"
        @archive="currentView = 'archive'"
        @trash="currentView = 'trash'"
        @reorder="onReorderGroups"
      />
    </v-navigation-drawer>

    <v-main>
      <v-container fluid class="pa-4">
        <TaskListView
          v-if="currentView === 'tasks'"
          :tasks="tasks"
          :loading="loadingTasks"
          :group-name="selectedGroup?.name ?? null"
          :group-description="selectedGroup?.description || null"
          @create="openCreateTask"
          @edit="openEditTask"
          @toggle="onToggleTask"
          @remove="onDeleteTask"
          @archive="onArchiveTask"
          @reorder="onReorderTasks"
        />
        <SettingsView
          v-else-if="currentView === 'settings'"
          :current-user="currentUser"
          @exported="notifyExported"
          @imported="onImported"
          @logout="onLogout"
          @error="notify"
          @notify="notify"
        />
        <ArchiveView
          v-else-if="currentView === 'archive'"
          :tasks="archiveTasks"
          :active-groups="groups"
          @restore="onUnarchiveTask"
          @remove="onRemoveArchivedTask"
        />

        <TrashView
          v-else-if="currentView === 'trash'"
          :groups="trashGroups"
          :tasks="trashTasks"
          :active-groups="groups"
          @restore="onRestoreTrash"
          @purge="onPurgeTrash"
          @empty="onEmptyTrash"
        />
        <MCPView v-else-if="currentView === 'mcp'" :current-user="currentUser" @notify="notify" />
        <PromptView v-else ref="promptView" @notify="notify" @error="notify" />
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

    <ContextMenu
      v-if="globalCtx"
      :items="globalCtx.items"
      :x="globalCtx.x"
      :y="globalCtx.y"
      @close="globalCtx = null"
    />
    </template>
  </v-app>
</template>

<style scoped>
/* 标题内容 flex 垂直居中：logo 按 inline 基线对齐会随行高偏移数像素，
   与右侧按钮图标不在同一水平线（Vuetify 4 的内容层为 __placeholder） */
:deep(.v-toolbar-title__placeholder) {
  display: flex;
  align-items: center;
}
.app-logo {
  width: 24px;
  height: 24px;
  margin-right: 8px;
}
</style>