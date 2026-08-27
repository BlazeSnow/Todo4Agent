# Todo4Agent 后端文案（中文，默认与回落语言）。
# 键为 kebab-case；带参用 {$name} 占位。结构对齐前端 src/i18n。

# ---------- 通用 ----------
db-error = 数据库错误: {$err}

# ---------- 认证 ----------
not-signed-in = 未登录或登录已失效
invalid-credentials = 用户名或密码错误
invalid-credentials-user = 用户名或密码错误：{$user}
username-empty = 用户名不能为空
password-too-short = 密码至少 4 位
registration-disabled = 注册已关闭
username-taken = 用户名已存在
new-password-too-short = 新密码至少 4 位
wrong-password = 原密码错误

# ---------- 分组 ----------
group-name-empty = 分组名不能为空
group-name-taken = 分组名已存在
no-fields-to-update = 没有需要更新的字段
no-group-rename = 系统分组「无分组」不可重命名
group-not-found = 分组不存在
no-group-lock = 系统分组「无分组」不可锁定，它是分组删除后任务的兜底去处
no-group-delete = 系统分组「无分组」不可删除
group-ids-empty = group_ids 不能为空

# ---------- 任务 ----------
task-title-empty = 任务标题不能为空
status-invalid = status 只能是 pending 或 done
task-not-found = 任务不存在
task-not-found-or-archived = 任务不存在或已归档
task-not-archived = 任务不在归档中
task-ids-empty = task_ids 不能为空

# ---------- 回收站 ----------
task-not-in-trash = 任务不在回收站
group-not-in-trash = 分组不在回收站

# ---------- 导入导出 / 设置 / 提示词 ----------
export-write-failed = 写入导出文件失败: {$err}
import-empty = 导入内容为空
port-range = 端口范围：1024-65535
prompt-save-error = 保存提示词结果异常

# ---------- MCP：启动与运行时消息 ----------
mcp-env-credentials = MCP 需要设置 TODO4AGENT_USERNAME 与 TODO4AGENT_PASSWORD 环境变量（运行 todo4agent help 查看接入说明）
verify-user-failed = 验证用户失败：{$err}
locked-err = 清单「{$name}」已锁定，Agent 无法编辑（请让用户在界面侧边栏分组菜单解锁）
import-locked = 文档包含已锁定的清单：{$names}（请让用户在界面导入或先解锁）
import-doc-invalid = 参数错误: doc 必须是任务清单文档 JSON: {$err}
unknown-tool = 未知工具: {$name}
password-changed-note = 密码已修改；请同步更新 MCP 客户端配置中的 TODO4AGENT_PASSWORD（当前连接不受影响，下次启动需用新密码）

# ---------- MCP：参数校验 ----------
arg-error-required = 参数错误: {$key} 必填
arg-error-required-nonempty = 参数错误: {$key} 必填且不能为空
arg-error-required-string = 参数错误: {$key} 必填且必须是字符串
arg-error-int = 参数错误: {$key} 必须是整数
arg-error-string = 参数错误: {$key} 必须是字符串
arg-error-bool = 参数错误: {$key} 必须是布尔值
arg-error-title-empty = 参数错误: title 不能为空
arg-error-status = 参数错误: status 只能是 pending 或 done
arg-error-due = 参数错误: due_at 必须是字符串或 null
arg-error-done = 参数错误: done 必填且必须是布尔值
arg-error-new-password-short = 参数错误: new_password 至少 4 位

# ---------- MCP：工具描述 ----------
tool-app-version = 查询应用版本号
tool-app-release = 查询应用发布页地址（GitHub Releases）
tool-db-path = 查询当前连接的数据库文件路径（本地 SQLite，可用环境变量 TODO4AGENT_DB 覆盖）
tool-group-list = 列出所有任务分组
tool-group-create = 创建任务分组；分组名不能重复
tool-group-rename = 重命名任务分组（可选同时更新分组描述）
tool-group-delete = 删除任务分组（组内任务含归档移入「无分组」；系统分组「无分组」不可删除）
tool-task-list = 列出任务；可按分组过滤，可选包含已归档
tool-task-create = 创建任务（默认状态 pending）
tool-task-update = 更新任务字段（只修改传入的字段）
tool-task-complete = 完成 / 取消完成一个任务（切换 done 状态）
tool-task-archive = 归档任务（从清单移入归档，界面「归档」页可查看与恢复）
tool-task-unarchive = 取消归档（任务回到原清单）
tool-task-delete = 删除任务
tool-task-export = 导出任务清单与提示词为 JSON 文档（与界面导出同构）
tool-task-import = 导入 JSON 文档（与 task_export 输出同构：同名分组并入、新分组新建，含 prompt 字段时提示词一并导入）
tool-user-password = 修改当前账号（启动凭据对应用户）的密码；成功后该用户的已登录会话全部失效，需同步更新客户端配置中的 TODO4AGENT_PASSWORD
tool-prompt-get = 读取当前用户的 Agent 提示词（协作规范，类似 AGENTS.md）；默认为空，content 为空表示尚未设置
tool-prompt-update = 全量更新当前用户的 Agent 提示词；建议先 prompt_get 获取当前内容，按需修改后整体写回；传空字符串为清空

# ---------- MCP：参数描述 ----------
tp-name-required = 分组名（必填）
tp-desc-purpose = 分组描述（可选）：说明该清单的用途
tp-group-id = 分组 id
tp-name-new = 新分组名（必填）
tp-desc-update = 分组描述（可选）：传入即更新，传空字符串清空
tp-group-id-required = 分组 id（必填）
tp-group-id-optional-all = 分组 id（可选，缺省返回全部）
tp-include-archived = 包含已归档任务（可选，默认 false 仅返回未归档）
tp-owning-group-required = 所属分组 id（必填）
tp-title-required = 任务标题（必填）
tp-desc-optional = 详细说明（可选）
tp-due-optional = 截止时间，ISO8601（可选）
tp-task-id-required = 任务 id（必填）
tp-move-group-id = 移动到的分组 id
tp-new-title = 新标题
tp-new-desc = 新说明
tp-new-status = 新状态
tp-new-due = 新截止时间；传 null 清空
tp-done = true 标记完成，false 恢复未完成（必填）
tp-doc = 任务清单文档（必填）：包含 version、exported_at 与 groups；每个分组含 name 与 tasks；每个任务含 title、description、status、due_at
tp-old-password = 当前密码（必填）
tp-new-password = 新密码，至少 4 位（必填）
tp-content = 新提示词全文；传空字符串清空（必填）
