/** 分组 */
export interface Group {
  id: number
  name: string
  sort_order: number
  created_at: string
  /** 回收站标记：非 null 表示已删除 */
  deleted_at: string | null
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
  /** 回收站标记：非 null 表示已删除 */
  deleted_at: string | null
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

/** 服务设置 */
export interface SettingsInfo {
  /** 配置的端口（保存值） */
  port: number
  /** 当前实际监听的端口 */
  effective_port: number
}

/** 导入结果统计 */
export interface ImportResult {
  groups_created: number
  groups_merged: number
  tasks_imported: number
  tasks_skipped: number
}

/** 认证状态 */
export interface AuthStatus {
  /** local = 未创建用户（本地模式）；users = 多用户模式 */
  mode: 'local' | 'users'
  user_id: number | null
  username: string | null
  /** 是否存在仍在使用初始默认密码的用户（登录页提示改密） */
  default_password: boolean
}