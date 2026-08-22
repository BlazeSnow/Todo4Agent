<#
.SYNOPSIS
  启动 Todo4Agent 开发环境：自动处理终端编码、检查依赖并一键运行。

.DESCRIPTION
  默认启动桌面开发模式（pnpm tauri dev），可选参数切换模式：
  -Web    纯 Web 模式：Vite 开发服务器（端口 3001）。后端需另开终端运行 .\run.ps1 -Serve。
  -Serve  仅启动 headless 后端（HTTP API + WebUI，端口 3000，被占用时顺延）。
  -Mcp    启动 MCP stdio 服务（供 Agent 连接操作任务清单）。
  -Build  打包当前平台安装包（pnpm tauri build）。
  首次运行会自动执行 pnpm install 安装前端依赖，可使用 -SkipInstall 跳过。
  脚本开头会将终端切换为 UTF-8 编码（chcp 65001），避免 GBK 终端下中文乱码。

.PARAMETER Web
  启动 Vite 开发服务器（纯前端，端口 3001）。

.PARAMETER Serve
  仅启动 headless 后端服务（端口 3000）。

.PARAMETER Mcp
  启动 MCP stdio 服务。

.PARAMETER Build
  打包当前平台安装包。

.PARAMETER SkipInstall
  跳过依赖安装（pnpm install）。

.EXAMPLE
  .\run.ps1                  # 桌面开发模式
  .\run.ps1 -Web             # 纯前端开发（配合 -Serve 使用）
  .\run.ps1 -Serve           # 仅后端 + WebUI
  .\run.ps1 -Mcp             # MCP 服务
  .\run.ps1 -Build           # 打包
#>

[CmdletBinding()]
param(
    [switch]$Web,
    [switch]$Serve,
    [switch]$Mcp,
    [switch]$Build,
    [switch]$SkipInstall
)

$ErrorActionPreference = 'Stop'

# 1. 终端切换为 UTF-8，避免 GBK 终端下中文乱码
chcp 65001 | Out-Null
try { [Console]::OutputEncoding = [System.Text.UTF8Encoding]::new() } catch { }

$RepoRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
Push-Location $RepoRoot
try {
    # 2. 检查基础依赖
    function Test-Command([string]$Name) {
        return [bool](Get-Command $Name -ErrorAction SilentlyContinue)
    }

    $missing = @()
    if (-not (Test-Command 'node')) { $missing += 'Node.js (>= 20)' }
    if (-not (Test-Command 'pnpm')) { $missing += 'pnpm' }
    if (-not (Test-Command 'cargo')) { $missing += 'Rust (cargo)' }
    if ($missing.Count -gt 0) {
        throw "缺少依赖：$($missing -join '、')。请先安装后再运行本脚本。"
    }

    # 3. 安装前端依赖（首次运行自动安装，可跳过）
    if (-not $SkipInstall -and -not (Test-Path (Join-Path $RepoRoot 'node_modules'))) {
        Write-Host '首次运行，正在安装前端依赖...'
        pnpm install
        if ($LASTEXITCODE -ne 0) { throw 'pnpm install 失败' }
    }

    # 4. 按模式启动
    if ($Web) {
        pnpm dev
    }
    elseif ($Serve) {
        pnpm backend
    }
    elseif ($Mcp) {
        pnpm mcp
    }
    elseif ($Build) {
        pnpm tauri build
    }
    else {
        pnpm tauri dev
    }
    if ($LASTEXITCODE -ne 0) { throw '运行失败' }
}
finally {
    Pop-Location
}