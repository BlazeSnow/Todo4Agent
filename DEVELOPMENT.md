# Todo for Agent 开发指南

> 本文档是开发操作指南，与 [AGENTS.md](./AGENTS.md)（需求规范）配套阅读。
> 两者冲突时以 AGENTS.md 为准；**AGENTS.md 禁止修改**。文档中的需求、接口、表结构均为草案，随开发推进更新。

## 1. 技术栈

| 层次    | 技术           | 说明                                       |
| ------- | -------------- | ------------------------------------------ |
| 桌面壳  | Tauri 2        | 跨平台桌面应用                             |
| 后端    | Rust（stable） | Tauri 逻辑、SQLite 数据层、MCP Server      |
| 前端    | Vue 3 + Vite   | 桌面与 WebUI 共用同一套前端                |
| UI 组件 | Vuetify        | 侧边栏分组、任务列表等                     |
| 数据库  | SQLite（本地） | 通过 `tauri-plugin-sql`（或 rusqlite）访问 |
| 协议    | MCP（stdio）   | 供 Agent 连接操作任务清单                  |

## 2. 开发环境

- Node.js ≥ 20、pnpm（或 npm）
- Rust stable（建议 1.77+），Windows 需安装 Microsoft C++ Build Tools
- Tauri CLI 2.x：`pnpm add -D @tauri-apps/cli`，命令为 `pnpm tauri ...`
- SQLite 不需要单独安装（桌面端由插件/依赖内置）

## 3. 终端编码（GBK / UTF-8）

本机（Windows）终端默认 GBK，Rust/Node 输出为 UTF-8，需统一处理：

- 所有源码文件保存为 UTF-8（无 BOM）。
- PowerShell 会话开头执行：
  ```powershell
  chcp 65001
  [Console]::OutputEncoding = [System.Text.UTF8Encoding]::new()
  ```
- Git Bash 执行：
  ```bash
  export LANG=zh_CN.UTF-8
  export LC_ALL=zh_CN.UTF-8
  ```
- 若 `cargo` / `tauri` 报错信息出现乱码，先检查会话编码再排查代码。
- 代码中读写文件统一以 UTF-8 编码进行；数据库导出的 JSON 也应为 UTF-8。

## 4. 目录结构（草案）

```
Todo4Agent/
├── src/                    # Vue 3 前端（桌面与 WebUI 共用）
│   ├── api/                # 调用后端的封装（tauri invoke / HTTP）
│   ├── components/
│   ├── views/
│   ├── plugins/vuetify.ts
│   └── main.ts
├── src-tauri/              # Rust 后端（Tauri 2）
│   ├── src/
│   │   ├── main.rs
│   │   ├── db.rs           # SQLite 初始化与访问
│   │   ├── mcp.rs          # MCP Server（stdio）
│   │   └── commands.rs     # tauri 命令（导出 JSON 等）
│   ├── capabilities/
│   └── tauri.conf.json
├── .github/workflows/      # 打包发布流水线
├── AGENTS.md               # 需求规范（禁止修改）
└── DEVELOPMENT.md          # 本文档
```

## 5. 常用命令

| 命令                         | 作用                                              |
| ---------------------------- | ------------------------------------------------- |
| `pnpm install`               | 安装前端依赖                                      |
| `pnpm dev`                   | 纯 Web 模式启动 Vite，**监听 3000 端口（WebUI）** |
| `pnpm tauri dev`             | 桌面开发模式                                      |
| `pnpm tauri build`           | 打包当前平台安装包                                |
| `cargo test`（src-tauri 下） | 运行 Rust 单元测试                                |

WebUI 与桌面端必须共享同一套功能与数据，禁止出现两套业务逻辑。Vite 的 `server.port` 固定为 3000。

## 6. 数据库设计（草案）

数据库文件位于 Tauri 的 app data 目录，首次启动自动建表、播种默认分组。

```sql
CREATE TABLE groups (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  name       TEXT NOT NULL UNIQUE,          -- 默认分组名："快速清单"
  sort_order INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE tasks (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  group_id    INTEGER NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
  title       TEXT NOT NULL,
  description TEXT NOT NULL DEFAULT '',
  status      TEXT NOT NULL DEFAULT 'pending',  -- pending / done
  due_at      TEXT,
  created_at  TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
);
```

导出 JSON 的格式约定：

```json
{
  "version": 1,
  "exported_at": "2026-08-22T12:00:00Z",
  "groups": [
    {
      "name": "快速清单",
      "tasks": [
        { "title": "示例任务", "description": "", "status": "pending", "due_at": null }
      ]
    }
  ]
}
```

## 7. MCP 接入（供 Agent 使用）

软件以 MCP Server（stdio 传输）暴露任务清单能力，Agent 通过 MCP 客户端连接。工具草案：

- `group_list` / `group_create` / `group_rename`
- `task_list`（可按分组过滤）/ `task_create` / `task_update` / `task_complete` / `task_delete`
- `task_export`（导出 JSON，与界面导出走同一实现）

约定：

- MCP Server 与桌面端访问同一个 SQLite 数据库文件，写入后界面应立即反映变化。
- 新增工具需同步更新 AGENTS.md 之外的使用说明（README 或 docs）。
- 所有工具必须返回结构化 JSON，错误信息要能让 Agent 直接理解并处理。

## 8. 发布（GitHub Actions）

- 流水线文件：`.github/workflows/release.yml`。
- 触发方式：推送形如 `v1.2.3` 的 tag 时打包发布正式版；推送 `v1.2.3-beta.1` 时以 **prerelease** 发布 beta 版。
- 流程：checkout → 安装 Node/Rust → `pnpm install` → `pnpm tauri build` → 将各平台安装包上传至 GitHub Release。
- 矩阵构建 Windows / macOS / Linux 三平台产物。
- 版本号以 tag 为准，`tauri.conf.json` 中版本与 tag 保持一致。

## 9. 开发里程碑

1. 初始化 Tauri 2 + Vue 3 + Vuetify 工程，Vite 绑定 3000 端口。
2. 实现 SQLite 数据层（建表、默认分组「快速清单」）及单元测试。
3. 任务清单界面：侧边栏分组管理、任务增删改查、完成状态。
4. WebUI（3000 端口）与桌面端功能对齐。
5. 实现导出 JSON。
6. 实现 MCP Server 并接入 Agent 测试。
7. 配置 GitHub Actions，验证正式版与 beta 版发布。