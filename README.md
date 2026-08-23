# Todo4Agent

为 Agent 设计的 MCP 任务清单：Agent 可以通过 MCP 直接管理你的任务，你也能在界面里手动维护。

## 功能

- 任务分组与任务管理：新建、编辑、完成、截止时间、排序
- 任务清单锁定：锁定后 Agent 仅能读取（界面编辑不受影响），可在任务卡片菜单或右键菜单一键切换
- 回收站：误删可恢复
- 多用户：每人的任务数据互相独立
- 导入 / 导出 JSON（含任务清单与提示词），方便备份与迁移
- 桌面应用（Windows / macOS / Linux）与浏览器 WebUI（默认 <http://127.0.0.1:3000>）
- WebUI 默认监听 `0.0.0.0`，局域网内其他设备可直接访问；可在「设置 → 服务」关闭对外访问（仅本机）或修改端口
- 新用户注册默认开启，可在「设置 → 用户」关闭
- MCP 服务：Agent 通过标准 MCP 协议直接操作任务清单
- 提示词：AGENTS.md 式的 Agent 协作规范，默认为空、由用户自行填写，界面可编辑/复制/清空，Agent 也可通过 MCP 读写（数据按用户隔离）

## 安装

在 [GitHub Releases](https://github.com/BlazeSnow/Todo4Agent/releases) 下载对应平台的安装包即可。
服务端口与对外监听可在「设置 → 服务」中修改（默认 3000 / 监听 0.0.0.0）。

## 命令行

| 命令 | 说明 |
| --- | --- |
| `todo4agent` | 启动桌面应用（后台同时提供 WebUI 服务） |
| `todo4agent serve` | 无界面运行 WebUI / HTTP API |
| `todo4agent mcp` | 启动 MCP Server（stdio，供 Agent 客户端连接） |
| `todo4agent help` | 查看完整帮助（含 MCP 客户端配置示例与初始账号说明） |
| `todo4agent version` | 查看版本号 |
| `--port <端口>` | 指定 WebUI/API 监听端口（1024-65535），本次运行有效、优先于设置页保存的端口，如 `todo4agent serve --port 8080`；适用于桌面与 serve 模式 |

## 首次使用

系统自带初始账号：

- 用户名：`admin`
- 密码：`admin123`

登录后请尽快在「设置 → 用户」中修改密码；也可以直接注册自己的新账号（注册按钮默认开启）。

## Agent 接入（MCP）

在支持 MCP 的 Agent 客户端中配置：

```json
{
  "mcpServers": {
    "todo4agent": {
      "command": "todo4agent",
      "args": ["mcp"],
      "env": {
        "TODO4AGENT_USERNAME": "你的用户名",
        "TODO4AGENT_PASSWORD": "你的密码"
      }
    }
  }
}
```

Agent 即可对你自己的任务清单执行增删改查、重排、导入导出等操作。软件界面「Agent 接入」页可一键复制客户端配置。可用工具：

| 工具 | 说明 |
| --- | --- |
| `app_version` / `app_release` | 查询应用版本号 / 发布页地址 |
| `group_list` / `group_create` / `group_rename` / `group_delete` | 任务分组管理（删除分组其下任务一并进回收站） |
| `task_list` / `task_create` / `task_update` | 任务查询与编辑（支持移动分组、改状态、截止时间） |
| `task_complete` / `task_delete` | 完成切换 / 删除（软删除进回收站） |
| `task_export` / `task_import` | 导出任务清单与提示词 JSON（与界面导出同构）/ 导入（同名分组并入，提示词随 `prompt` 字段迁移） |
| `user_password` | 修改当前账号密码（原密码 + 新密码；改后该用户已登录会话失效，需同步更新客户端 env 中的 `TODO4AGENT_PASSWORD`） |
| `prompt_get` / `prompt_update` | 读取 / 全量更新当前用户的 Agent 提示词（默认为空；`prompt_update` 传空字符串即清空） |

> 凭据说明：MCP 启动时必须通过 `TODO4AGENT_USERNAME` / `TODO4AGENT_PASSWORD` 指定并验证账号
> （首次运行数据库会自动创建初始账号 admin / admin123），验证失败将拒绝启动。
>
> 锁定说明：任务清单锁定后（界面任务卡片 ⋮ 菜单或右键菜单切换），上表中的写操作
> （分组/任务增删改、导入）会被拒绝并提示，读取类工具（列表、导出等）不受影响。

## 数据

任务数据保存在本机 SQLite 数据库中，界面「设置 → 数据」可导出/导入 JSON，并可在系统文件管理器中定位数据库文件。

## 文档

- 开发者文档：[DEVELOPMENT.md](DEVELOPMENT.md)
- 更新日志：[CHANGELOG.md](CHANGELOG.md)
- 许可证：[GNU AGPL-3.0](LICENSE)
