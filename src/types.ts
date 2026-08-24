/** 分组 */
export interface Group {
  id: number
  name: string
  /** 分组描述：说明该清单的用途（如给 Agent 的使用备注），可为空 */
  description: string
  sort_order: number
  created_at: string
  /** 回收站标记：非 null 表示已删除 */
  deleted_at: string | null
  /** 清单锁定：锁定后 Agent 无法通过 MCP 编辑该清单，界面编辑不受影响 */
  locked: boolean
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
  /** 归档标记：非 null 表示已归档（值为归档时间） */
  archived_at: string | null
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
  /** 用户提示词（Agent 协作规范）；未设置为 null，旧版文件无此字段 */
  prompt?: string | null
  groups: {
    name: string
    /** 分组描述；旧版导出文件无此字段 */
    description?: string
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
  /** 是否对外（0.0.0.0）开放 WebUI */
  webui_lan: boolean
  /** 是否允许注册新账号 */
  allow_register: boolean
  /** 数据库文件路径 */
  db_path: string
}

/** 导入结果统计 */
export interface ImportResult {
  groups_created: number
  groups_merged: number
  tasks_imported: number
  tasks_skipped: number
  /** 是否导入/更新了提示词（文档含 prompt 字段时） */
  prompt_imported: boolean
}

/** Agent 提示词（协作规范，类似 AGENTS.md；默认为空） */
export interface PromptInfo {
  content: string
  /** 是否为默认状态（未自定义 / 已清空） */
  is_default: boolean
  updated_at: string | null
}

/** 认证状态 */
export interface AuthStatus {
  user_id: number | null
  username: string | null
  /** 是否存在仍在使用初始默认密码的用户（登录页提示改密） */
  default_password: boolean
  /** 是否允许注册新账号（关闭时登录页隐藏注册按钮） */
  allow_register: boolean
}