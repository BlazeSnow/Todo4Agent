# Todo4Agent

为 Agent 设计的 MCP 任务清单：Agent 可以通过 MCP 直接管理你的任务，你也能在界面里手动维护。

## 功能

- 任务分组与任务管理：新建、编辑、完成、截止时间、排序
- 回收站：误删可恢复
- 多用户：每人的任务数据互相独立
- 导入 / 导出 JSON，方便备份与迁移
- 桌面应用（Windows / macOS / Linux）与浏览器 WebUI（默认 <http://127.0.0.1:3000>）
- WebUI 默认监听 `0.0.0.0`，局域网内其他设备可直接访问；可在「设置 → 服务」关闭对外访问（仅本机）或修改端口
- 新用户注册默认开启，可在「设置 → 用户」关闭
- MCP 服务：Agent 通过标准 MCP 协议直接操作任务清单

## 安装

在 [GitHub Releases](https://github.com/BlazeSnow/Todo4Agent/releases) 下载对应平台的安装包即可。
服务端口与对外监听可在「设置 → 服务」中修改（默认 3000 / 监听 0.0.0.0）。

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
| `task_export` / `task_import` | 导出任务清单 JSON（与界面导出同构）/ 导入（同名分组并入） |

> 凭据说明：MCP 启动时必须通过 `TODO4AGENT_USERNAME` / `TODO4AGENT_PASSWORD` 指定并验证账号
> （首次运行数据库会自动创建初始账号 admin / admin123），验证失败将拒绝启动。

## 数据

任务数据保存在本机 SQLite 数据库中，界面「设置 → 数据」可导出/导入 JSON。

## 文档

- 开发者文档：[DEVELOPMENT.md](DEVELOPMENT.md)
- 更新日志：[CHANGELOG.md](CHANGELOG.md)
- 许可证：[GNU AGPL-3.0](LICENSE)
