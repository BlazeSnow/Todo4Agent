import { i18n, locale } from './i18n'
import type {
  AuthStatus,
  ExportDoc,
  Group,
  ImportResult,
  PromptInfo,
  SettingsInfo,
  Task,
  TaskInput,
  TaskUpdate,
} from './types'

const TOKEN_KEY = 'todo4agent_token'

export function getToken(): string | null {
  return localStorage.getItem(TOKEN_KEY)
}

export function setToken(token: string | null): void {
  if (token) localStorage.setItem(TOKEN_KEY, token)
  else localStorage.removeItem(TOKEN_KEY)
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    // 后端错误信息按界面语言返回
    'Accept-Language': locale.value,
  }
  const token = getToken()
  if (token) headers.Authorization = `Bearer ${token}`
  // 禁用浏览器缓存：认证状态与数据接口必须实时（避免 GET 被启发式缓存）
  const res = await fetch(`/api${path}`, { cache: 'no-store', headers, ...init })
  if (res.status === 401 && !path.startsWith('/auth/')) {
    // 会话失效：清除 token，由 App 切换到登录界面
    setToken(null)
    const err = new Error(i18n.global.t('app.sessionExpired')) as Error & { status: number }
    err.status = 401
    throw err
  }
  if (!res.ok) {
    let message = `HTTP ${res.status}`
    try {
      const body = await res.json()
      if (body && typeof body.error === 'string') message = body.error
    } catch {
      // 忽略非 JSON 错误体
    }
    throw new Error(message)
  }
  return res.json() as Promise<T>
}

// ---------- 认证 / 用户 ----------

export function authStatus(): Promise<AuthStatus> {
  return request<AuthStatus>('/auth/status')
}

export function authLogin(username: string, password: string) {
  return request<{ token: string; user_id: number; username: string }>('/auth/login', {
    method: 'POST',
    body: JSON.stringify({ username, password }),
  })
}

export function authRegister(username: string, password: string) {
  return request<{ token: string; user_id: number; username: string }>('/auth/register', {
    method: 'POST',
    body: JSON.stringify({ username, password }),
  })
}

export function authLogout(): Promise<{ ok: boolean }> {
  return request<{ ok: boolean }>('/auth/logout', { method: 'POST' })
}

export function authChangePassword(oldPassword: string, newPassword: string): Promise<{ ok: boolean }> {
  return request<{ ok: boolean }>('/auth/password', {
    method: 'POST',
    body: JSON.stringify({ old_password: oldPassword, new_password: newPassword }),
  })
}

export async function listGroups(): Promise<Group[]> {
  const r = await request<{ groups: Group[] }>('/groups')
  return r.groups
}

export function createGroup(name: string, description: string): Promise<Group> {
  return request<Group>('/groups', {
    method: 'POST',
    body: JSON.stringify({ name, description }),
  })
}

/** 更新分组：重命名 / 修改描述（只传需要改的字段） */
export function updateGroup(
  id: number,
  patch: { name?: string; description?: string },
): Promise<Group> {
  return request<Group>(`/groups/${id}`, { method: 'PATCH', body: JSON.stringify(patch) })
}

/** 切换清单锁定：锁定后 Agent 无法通过 MCP 编辑该清单，界面编辑不受影响 */
export function setGroupLocked(id: number, locked: boolean): Promise<Group> {
  return request<Group>(`/groups/${id}`, { method: 'PATCH', body: JSON.stringify({ locked }) })
}

/** 按给定顺序重排全部分组 */
export function reorderGroups(groupIds: number[]): Promise<{ ok: boolean }> {
  return request<{ ok: boolean }>('/groups/reorder', {
    method: 'POST',
    body: JSON.stringify({ group_ids: groupIds }),
  })
}

export function deleteGroup(id: number): Promise<{ ok: boolean }> {
  return request<{ ok: boolean }>(`/groups/${id}`, { method: 'DELETE' })
}

export async function listTasks(groupId?: number): Promise<Task[]> {
  const q = groupId != null ? `?group_id=${groupId}` : ''
  const r = await request<{ tasks: Task[] }>(`/tasks${q}`)
  return r.tasks
}

export function createTask(input: TaskInput): Promise<Task> {
  return request<Task>('/tasks', { method: 'POST', body: JSON.stringify(input) })
}

export function updateTask(id: number, patch: TaskUpdate): Promise<Task> {
  return request<Task>(`/tasks/${id}`, { method: 'PATCH', body: JSON.stringify(patch) })
}

// ---------- 归档 ----------

export async function listArchive(): Promise<Task[]> {
  const r = await request<{ tasks: Task[] }>('/archive')
  return r.tasks
}

/** 归档任务（从清单移入归档） */
export function archiveTask(id: number): Promise<{ ok: boolean }> {
  return request<{ ok: boolean }>(`/tasks/${id}/archive`, { method: 'POST' })
}

/** 取消归档（回到原清单） */
export function unarchiveTask(id: number): Promise<{ ok: boolean }> {
  return request<{ ok: boolean }>(`/tasks/${id}/unarchive`, { method: 'POST' })
}

export function deleteTask(id: number): Promise<{ ok: boolean }> {
  return request<{ ok: boolean }>(`/tasks/${id}`, { method: 'DELETE' })
}

/** 按给定顺序重排某分组内的任务 */
export function reorderTasks(groupId: number, taskIds: number[]): Promise<{ ok: boolean }> {
  return request<{ ok: boolean }>(`/tasks/reorder/${groupId}`, {
    method: 'POST',
    body: JSON.stringify({ task_ids: taskIds }),
  })
}

// ---------- 回收站 ----------

export interface TrashData {
  groups: Group[]
  tasks: Task[]
}

export async function listTrash(): Promise<TrashData> {
  return request<TrashData>('/trash')
}

export function restoreTask(id: number): Promise<{ ok: boolean }> {
  return request<{ ok: boolean }>(`/tasks/${id}/restore`, { method: 'POST' })
}

export function purgeTask(id: number): Promise<{ ok: boolean }> {
  return request<{ ok: boolean }>(`/tasks/${id}/purge`, { method: 'DELETE' })
}

/** 恢复分组；原名被现有分组占用时会自动重命名，renamed_to 为新名字 */
export function restoreGroup(id: number): Promise<{ ok: boolean; renamed_to?: string }> {
  return request(`/groups/${id}/restore`, { method: 'POST' })
}

export function purgeGroup(id: number): Promise<{ ok: boolean }> {
  return request<{ ok: boolean }>(`/groups/${id}/purge`, { method: 'DELETE' })
}

export function emptyTrash(): Promise<{ ok: boolean }> {
  return request<{ ok: boolean }>('/trash', { method: 'DELETE' })
}

export async function exportDoc(): Promise<ExportDoc> {
  return request<ExportDoc>('/export')
}

/** 导入 JSON（同名分组并入，新分组新建） */
export function importDoc(doc: ExportDoc): Promise<ImportResult> {
  return request<ImportResult>('/import', {
    method: 'POST',
    body: JSON.stringify(doc),
  })
}

// ---------- 提示词 ----------

/** 获取当前用户提示词（未自定义时返回默认内容） */
export function getPrompt(): Promise<PromptInfo> {
  return request<PromptInfo>('/prompt')
}

/** 全量保存提示词（与 MCP prompt_update 同一实现） */
export function savePrompt(content: string): Promise<PromptInfo> {
  return request<PromptInfo>('/prompt', {
    method: 'PUT',
    body: JSON.stringify({ content }),
  })
}

// ---------- 设置 ----------

export function getSettings(): Promise<SettingsInfo> {
  return request<SettingsInfo>('/settings')
}

/** 保存服务设置：只更新传入的字段 */
export function updateSettings(input: {
  port?: number
  webui_lan?: boolean
  allow_register?: boolean
}): Promise<SettingsInfo> {
  return request<SettingsInfo>('/settings', {
    method: 'PATCH',
    body: JSON.stringify(input),
  })
}

/** 在系统文件管理器中打开数据库文件位置（后端执行），返回文件路径 */
export function openDbLocation(): Promise<{ ok: boolean; path: string }> {
  return request<{ ok: boolean; path: string }>('/settings/db-location', { method: 'POST' })
}

/** 重启应用：桌面模式整体重启（窗口重建），serve 模式停机后自动拉起 */
export function restartApp(): Promise<{ restarting: boolean }> {
  return request<{ restarting: boolean }>('/app/restart', { method: 'POST' })
}

/** 导出 JSON 并触发浏览器下载 */
export function downloadExport(doc: ExportDoc): void {
  const blob = new Blob([JSON.stringify(doc, null, 2)], { type: 'application/json' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `todo4agent-export-${new Date().toISOString().slice(0, 10)}.json`
  a.click()
  URL.revokeObjectURL(url)
}