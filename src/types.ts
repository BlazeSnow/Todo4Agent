/** 分组 */
export interface Group {
  id: number
  name: string
  sort_order: number
  created_at: string
}

/** 任务状态 */
export type TaskStatus = 'pending' | 'done'

/** 任务 */
export interface Task {
  id: number
  group_id: number
  title: string
  description: string
  status: TaskStatus
  due_at: string | null
  created_at: string
  updated_at: string
  /** 手动排序序号（越小越靠前） */
  sort_order: number
}

/** 建任务输入 */
export interface TaskInput {
  group_id: number
  title: string
  description: string
  due_at: string | null
}

/** 修改任务输入（字段可选） */
export interface TaskUpdate {
  group_id?: number
  title?: string
  description?: string
  status?: TaskStatus
  due_at?: string | null
}

/** 导出 JSON 结构（与后端约定一致） */
export interface ExportDoc {
  version: number
  exported_at: string
  groups: {
    name: string
    tasks: {
      title: string
      description: string
      status: TaskStatus
      due_at: string | null
    }[]
  }[]
}