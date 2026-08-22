import type { AuthStatus, ExportDoc, Group, Task, TaskInput, TaskUpdate } from './types'

const TOKEN_KEY = 'todo4agent_token'

export function getToken(): string | null {
  return localStorage.getItem(TOKEN_KEY)
}

export function setToken(token: string | null): void {
  if (token) localStorage.setItem(TOKEN_KEY, token)
  else localStorage.removeItem(TOKEN_KEY)
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const headers: Record<string, string> = { 'Content-Type': 'application/json' }
  const token = getToken()
  if (token) headers.Authorization = `Bearer ${token}`
  // 禁用浏览器缓存：认证状态与数据接口必须实时（避免 GET 被启发式缓存）
  const res = await fetch(`/api${path}`, { cache: 'no-store', headers, ...init })
  if (res.status === 401 && !path.startsWith('/auth/')) {
    // 会话失效：清除 token，由 App 切换到登录界面
    setToken(null)
    const err = new Error('登录已失效，请重新登录') as Error & { status: number }
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

export function createGroup(name: string): Promise<Group> {
  return request<Group>('/groups', { method: 'POST', body: JSON.stringify({ name }) })
}

export function renameGroup(id: number, name: string): Promise<Group> {
  return request<Group>(`/groups/${id}`, { method: 'PATCH', body: JSON.stringify({ name }) })
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

export function restoreGroup(id: number): Promise<{ ok: boolean }> {
  return request<{ ok: boolean }>(`/groups/${id}/restore`, { method: 'POST' })
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

// ---------- 设置 ----------

export function getSettings(): Promise<SettingsInfo> {
  return request<SettingsInfo>('/settings')
}

/** 保存端口配置（重启应用后生效） */
export function updateSettings(port: number): Promise<{ port: number }> {
  return request<{ port: number }>('/settings', {
    method: 'PATCH',
    body: JSON.stringify({ port }),
  })
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