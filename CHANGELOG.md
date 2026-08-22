# Changelog

本仓库的发布历史，格式参考 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)。
每个版本章节标题为 `## [<tag>] - <日期>`（tag 需与发布 tag 一致，如 `v1.0.0-beta.1`）；
发布时 GitHub Actions 会从本文档提取对应章节作为 Release 说明。

## [v1.0.0-beta.1] - 2026-08-22

### Added

- 多用户系统：用户名密码登录、注册新用户、修改密码、退出登录，各用户数据独立隔离
- 初始用户 admin（默认密码 admin123，登录后应尽快修改）
- MCP 服务（stdio）：分组/任务的增删改查、排序、恢复与导出共 9 个工具；
  支持通过环境变量 `TODO4AGENT_USERNAME` / `TODO4AGENT_PASSWORD` 指定用户身份并验证
- 任务分组管理：侧边栏分组、重命名、删除、排序（上移/下移）
- 任务管理：新建/编辑/删除、完成状态、截止时间、按截止时间/标题排序
- 回收站：删除的任务与分组可恢复、彻底删除、一键清空
- 数据导入/导出 JSON（导入支持同名分组合并）
- 服务设置：WebUI/API 端口可配置（默认 3000）
- Tauri 2 桌面应用 + WebUI（3000 端口）；应用图标（T4A，主题色）
- 登录会话持久化：重启应用后无需重新登录

### Changed

- 界面主题色统一为 #00a862
- 侧边栏结构：分组、回收站、Agent 接入（MCP）、设置（含选中高亮）

### Security

- 密码以加盐 SHA-256 存储；API 需 Bearer token 访问