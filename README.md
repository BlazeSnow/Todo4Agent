# Todo4Agent

为 Agent 设计的 MCP 任务清单：Agent 可以通过 MCP 直接管理你的任务，你也能在界面里手动维护。

## 功能

- 任务分组与任务管理：新建、编辑、完成、截止时间、排序
- 回收站：误删可恢复
- 多用户：每人的任务数据互相独立
- 导入 / 导出 JSON，方便备份与迁移
- 桌面应用（Windows / macOS / Linux）与浏览器 WebUI（默认 http://127.0.0.1:3000）

## 安装

在 [GitHub Releases](https://github.com/BlazeSnow/Todo4Agent/releases) 下载对应平台的安装包即可。
浏览器访问 WebUI 时服务端口可在「设置 → 服务」中修改（默认 3000）。

## 首次使用

系统自带初始账号：

- 用户名：`admin`
- 密码：`admin123`

登录后请尽快在「设置 → 用户」中修改密码。

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

Agent 即可对你自己的任务清单执行增删改查、重排、导出等操作。

## 数据

任务数据保存在本机 SQLite 数据库中，界面「设置 → 数据」可导出/导入 JSON。

## 文档

- 开发者文档：[DEVELOPMENT.md](DEVELOPMENT.md)
- 更新日志：[CHANGELOG.md](CHANGELOG.md)

## 许可证

[GNU AGPL-3.0](LICENSE)