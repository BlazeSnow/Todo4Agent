import type { ExportDoc, Group, Task, TaskInput, TaskUpdate } from './types'

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`/api${path}`, {
    headers: { 'Content-Type': 'application/json' },
    ...init,
  })
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

export async function exportDoc(): Promise<ExportDoc> {
  return request<ExportDoc>('/export')
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