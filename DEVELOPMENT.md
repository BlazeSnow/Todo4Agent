# Todo for Agent 开发指南

> 本文档是开发操作指南，与 [AGENTS.md](./AGENTS.md)（需求规范）配套阅读。
> 两者冲突时以 AGENTS.md 为准；**AGENTS.md 禁止修改**。文档中的需求、接口、表结构均为草案，随开发推进更新。

## 1. 技术栈

| 层次    | 技术           | 说明                                           |
| ------- | -------------- | ---------------------------------------------- |
| 桌面壳  | Tauri 2        | 跨平台桌面应用，含系统托盘（tray-icon）        |
| 后端    | Rust（stable） | Tauri 逻辑、SQLite 数据层、MCP Server          |
| 前端    | Vue 3 + Vite   | 桌面与 WebUI 共用同一套前端                    |
| UI 组件 | Vuetify        | 主题跟随系统深浅色（`defaultTheme: 'system'`） |
| 数据库  | SQLite（本地） | rusqlite 直接访问（bundled，无需单独安装）     |
| 协议    | MCP（stdio）   | 供 Agent 连接操作任务清单，环境变量凭据认证    |

## 2. 开发环境

- Node.js ≥ 20、pnpm（或 npm）
- Rust stable（建议 1.77+），Windows 需安装 Microsoft C++ Build Tools
- Tauri CLI 2.x：`pnpm add -D @tauri-apps/cli`，命令为 `pnpm tauri ...`
- SQLite 不需要单独安装（桌面端由依赖内置）

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

## 4. 目录结构

```
Todo4Agent/
├── src/                    # Vue 3 前端（桌面与 WebUI 共用）
│   ├── api.ts              # 后端 HTTP API 封装
│   ├── types.ts
│   ├── main.ts             # Vuetify 初始化（主题跟随系统深浅色）
│   ├── App.vue             # 布局、认证门控、全局右键菜单兜底
│   └── components/         # 分组/任务/登录/设置/MCP/回收站视图、
│                           # 各对话框、ContextMenu（自定义右键菜单）
├── public/                 # 静态资源：icon.svg（WebUI favicon 与顶栏 logo）
├── src-tauri/              # Rust 后端（Tauri 2）
│   ├── src/
│   │   ├── main.rs         # 入口：tauri 桌面（含系统托盘）/ serve / mcp / help / version 模式
│   │   ├── auth.rs         # 密码盐与哈希
│   │   ├── api/            # axum HTTP API（认证、分组、任务、回收站、设置）
│   │   ├── db/             # SQLite 数据层（多用户、会话、导出）
│   │   └── mcp/            # MCP Server（stdio，环境变量认证）
│   ├── capabilities/       # Tauri 权限配置
│   └── tauri.conf.json     # 打包配置（Windows 安装包中文化）
├── ci/                     # 发布辅助脚本（版本一致性检查、Linux 依赖）
├── run.ps1                 # 开发环境一键启动（编码处理 + 模式切换）
├── version.ps1             # 将 package.json 版本同步到 tauri.conf.json / Cargo.toml / Cargo.lock
├── tag.ps1                 # 按版本打 tag，触发 GitHub Actions 发布
└── AGENTS.md / DEVELOPMENT.md
```

## 5. 常用命令

| 命令                                     | 作用                                                                         |
| ---------------------------------------- | ---------------------------------------------------------------------------- |
| `pnpm install`                           | 安装前端依赖                                                                 |
| `pnpm dev`                               | 纯 Web 模式启动 Vite（端口 3001，`/api` 代理到 127.0.0.1:3000）              |
| `pnpm backend`                           | headless 后端：Rust HTTP 服务 + WebUI（端口 3000）                           |
| `pnpm mcp`                               | 启动 MCP stdio 服务（供 Agent 连接，等价于 `todo4agent mcp`）                |
| `cargo run --manifest-path src-tauri/Cargo.toml -- help` | 查看后端 CLI 帮助（运行模式、MCP 配置示例、数据库路径）      |
| `cargo run --manifest-path src-tauri/Cargo.toml -- serve --port 8080` | 指定端口无界面启动（`--port` 本次运行有效，优先于设置页配置） |
| `pnpm tauri dev`                         | 桌面开发模式（后端 3000 + 窗口加载 Vite 3001）                               |
| `pnpm tauri build`                       | 打包当前平台安装包                                                           |
| `cargo test`（src-tauri 下）             | 运行 Rust 单元测试                                                           |
| `.\run.ps1 [-Web\|-Serve\|-Mcp\|-Build]` | 开发一键启动：默认桌面模式，参数切换模式，自动处理终端编码                   |
| `.\version.ps1`                          | 将 package.json 版本同步到 tauri.conf.json 与 Cargo.toml / Cargo.lock（含 MSI 数字版本） |
| `.\tag.ps1`                              | 按版本号打 tag 并推送，触发发布流水线（`-beta.N` 后缀为 prerelease）         |

WebUI 与桌面端必须共享同一套功能与数据，禁止出现两套业务逻辑。后端 HTTP 服务固定监听
3000 端口（被占用时顺延至 3010），生产环境 WebUI 即该端口；开发环境 Vite 监听 3001，
将 `/api` 代理到后端。桌面端窗口在开发模式加载 Vite（3001），生产模式加载后端 WebUI（3000）。

## 6. 数据库设计（草案）

数据库文件位于平台数据目录（Windows 为 `%LOCALAPPDATA%\Todo4Agent\todo.db`），
可用环境变量 `TODO4AGENT_DB` 指定其他位置；首次启动自动建表、播种默认分组
「快速清单」与初始用户 admin（默认密码 admin123，对应 `db/mod.rs` 的 `seed_default_admin`）。

```sql
CREATE TABLE users (
  id               INTEGER PRIMARY KEY AUTOINCREMENT,
  username         TEXT NOT NULL UNIQUE,
  salt             TEXT NOT NULL,
  password_hash    TEXT NOT NULL,  -- pbkdf2$<迭代>$<盐>$<哈希>（旧格式登录时透明升级）
  created_at       TEXT NOT NULL,
  default_password INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE groups (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  name       TEXT NOT NULL,        -- 唯一性见下方部分唯一索引（按用户、不含回收站）
  sort_order INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  deleted_at TEXT,          -- 非空表示已入回收站
  user_id    INTEGER        -- 所属用户；旧本地模式库由播种的 admin 接管 NULL 行
);

-- 分组名唯一性按用户生效，且不含回收站中的分组（软删除不占名）
CREATE UNIQUE INDEX idx_groups_user_name ON groups(user_id, name) WHERE deleted_at IS NULL;

CREATE TABLE tasks (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  group_id    INTEGER NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
  title       TEXT NOT NULL,
  description TEXT NOT NULL DEFAULT '',
  status      TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'done')),
  due_at      TEXT,
  created_at  TEXT NOT NULL,
  updated_at  TEXT NOT NULL,
  sort_order  INTEGER NOT NULL DEFAULT 0,
  deleted_at  TEXT          -- 非空表示已入回收站（软删除）
);

CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE sessions (token TEXT PRIMARY KEY, user_id INTEGER NOT NULL);
```

说明：

- 删除统一走软删除（`deleted_at`），回收站可恢复；清空回收站才物理删除。
  恢复分组遇同名冲突时自动顺延为「原名 (2)」。
- 认证为用户名 + 盐 + PBKDF2-HMAC-SHA256 哈希（`src/auth.rs`，旧库单轮 SHA-256
  格式在下次登录成功时透明升级）；登录签发会话 token（`sessions`），
  修改密码后吊销该用户的其他会话。
- 应用启动即播种初始用户 admin；admin 创建时接管旧本地模式库的无主数据
  （`groups.user_id IS NULL`），已有用户的数据库不会重复播种默认分组。
- 导出 JSON 的格式约定（与界面/MCP 导出同构）：

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

软件以 MCP Server（stdio 传输）暴露任务清单能力，Agent 通过 MCP 客户端连接。工具：

- `app_version` / `app_release`：查询版本号 / 发布页地址
- `group_list` / `group_create` / `group_rename` / `group_delete`（删除分组其下任务一并进回收站）
- `task_list`（可按分组过滤）/ `task_create` / `task_update` / `task_complete` / `task_delete`
- `task_export`（导出 JSON，与界面导出走同一实现）/ `task_import`（导入 JSON，同名分组并入）

认证与环境变量：

- 通过 `TODO4AGENT_USERNAME` / `TODO4AGENT_PASSWORD` 指定真实用户凭据，启动时校验，
  缺失任一或校验失败将以非零码退出（凭据必填；首次运行数据库会自动创建初始账号 admin）。
- 客户端配置示例（ZCode / Claude Desktop 通用格式）：

```json
{
  "mcpServers": {
    "todo4agent": {
      "command": "todo4agent",
      "args": ["mcp"],
      "env": { "TODO4AGENT_USERNAME": "你的用户名", "TODO4AGENT_PASSWORD": "你的密码" }
    }
  }
}
```

约定：

- MCP Server 与桌面端访问同一个 SQLite 数据库文件，写入后界面应能立即反映变化（界面刷新按钮会重载任务列表）。
- 新增工具需同步更新使用说明（README 或 docs）。
- 所有工具必须返回结构化 JSON，错误信息要能让 Agent 直接理解并处理。

## 8. 发布（GitHub Actions）

- 流水线文件：`.github/workflows/release.yml`。
- 触发方式：推送形如 `v1.2.3` 的 tag 时打包发布正式版；推送 `v1.2.3-beta.1` 时以 **prerelease** 发布 beta 版。
- 版本号唯一来源为 package.json 的 `version`（`X.Y.Z[-beta.N]`）、`baseVersion`（tag 前缀，
  如 `v1.0.0`）与 `msiVersion`（Windows MSI 发布序列号 `X.Y.N`，MSI 版本比较只看前三段且
  不允许字母，故独立于软件版本单调递增）：`.\version.ps1` 将其同步到 tauri.conf.json、
  Cargo.toml 与 Cargo.lock；流水线内 `ci/check-version.sh` 校验 tag 与各版本位置一致。
- 矩阵构建 Windows / macOS / Linux 三平台产物；Linux 构建依赖见 `ci/linux-deps.sh`
  （含系统托盘所需的 libayatana-appindicator3-dev）。
- Windows 安装包（MSI/NSIS）为中文界面（`wix.language: zh-CN`、`nsis.languages: SimpChinese`），
  发布版为 GUI 子系统，启动不弹出终端。

## 9. 开发里程碑

1. 初始化 Tauri 2 + Vue 3 + Vuetify 工程，后端绑定 3000 端口、Vite dev 3001。
2. 实现 SQLite 数据层（建表、默认分组「快速清单」）及单元测试。
3. 任务清单界面：侧边栏分组管理、任务增删改查、完成状态。
4. WebUI（3000 端口）与桌面端功能对齐。
5. 实现导出 JSON。
6. 实现 MCP Server 并接入 Agent 测试。
7. 配置 GitHub Actions，验证正式版与 beta 版发布。
8. 多用户认证（初始 admin、会话 token、注册/登录/改密）与回收站。
9. 系统托盘：关闭窗口驻留后台，托盘菜单显示/退出。
10. 自定义右键菜单（任务/分组快捷操作），刷新同步 MCP 等外部修改。
11. 适配系统深色模式（跟随系统，含任务列表等自写组件）。
12. WebUI 使用软件图标（favicon 与顶栏 logo），Windows 安装包中文化。
