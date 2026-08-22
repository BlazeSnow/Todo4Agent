<#
.SYNOPSIS
  根据 package.json 中的 version 字段创建并推送 git tag，触发 GitHub Actions 自动发布 Release。

.DESCRIPTION
  仓库的 Release 工作流在推送 v* 格式的 tag 时自动打包并创建 Release（vX.Y.Z-beta.N
  格式的 tag 会标为 Prerelease）。
  本脚本默认读取仓库根目录 package.json 的 version 字段作为 tag，-beta.N 后缀原样保留
  （例如 version 为 1.0.0-beta.1 时打 v1.0.0-beta.1）；也可以用 -Tag 显式指定。
  正式版还是测试版由版本号本身决定：含 -beta.N 后缀即为测试版，否则为正式版。
  执行前会打印将要执行的操作并要求输入 y 确认，输入其他内容则直接取消、不做任何改动。

.PARAMETER Tag
  要打的版本号，格式 vX.Y.Z 或 vX.Y.Z-beta.N，例如 v1.5.0、v1.5.0-beta.2。
  缺省时从 package.json 的 version 字段读取并原样使用（-beta.N 保留）。

.PARAMETER NoPush
  只创建 tag，不推送到远程（不会触发任何工作流）。

.EXAMPLE
  .\tag.ps1                       # 按 package.json 的 version 打 tag（如 v1.5.0 或 v1.5.0-beta.1）
  .\tag.ps1 -Tag v1.5.0           # 显式指定版本号
  .\tag.ps1 -NoPush               # 只创建本地 tag
#>

[CmdletBinding()]
param(
    [string]$Tag,
    [switch]$NoPush
)

$ErrorActionPreference = 'Stop'

$RepoRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
Push-Location $RepoRoot
try {
    # 1. 确定版本号：优先用 -Tag，否则从 VERSION 文件读取（发版前先运行 version.ps1 同步版本）
    if (-not $Tag) {
        $version = (Get-Content (Join-Path $RepoRoot 'VERSION') -Raw -Encoding UTF8).Trim()
        if (-not $version) {
            throw 'VERSION 文件中缺少版本号，请先填写或用 -Tag 显式指定。'
        }
        if ($version -notmatch '^\d+\.\d+\.\d+(-beta\.\d+)?$') {
            throw "VERSION 文件格式不正确：$version（应为 X.Y.Z 或 X.Y.Z-beta.N，例如 1.0.0、1.0.0-beta.2）"
        }
        $Tag = 'v' + $version
        Write-Host "从 VERSION 读取版本：$version"
    }

    if ($Tag -notmatch '^v\d+\.\d+\.\d+(-beta\.\d+)?$') {
        throw "tag 格式不正确：$Tag（应为 vX.Y.Z 或 vX.Y.Z-beta.N，例如 v1.5.0、v1.5.0-beta.2）"
    }
    $Beta = $Tag -match '-beta\.\d+$'

    # 3. 检查本地和远程是否已存在该 tag
    git tag --list $Tag | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'git tag --list 执行失败' }
    if (git tag --list $Tag) {
        throw "本地已存在 tag：$Tag"
    }

    git ls-remote --tags origin $Tag | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'git ls-remote 执行失败' }
    if (git ls-remote --tags origin $Tag) {
        throw "远程已存在 tag：$Tag"
    }

    # 4. 打印将要执行的操作，要求输入 y 确认
    $currentBranch = git rev-parse --abbrev-ref HEAD
    if ($LASTEXITCODE -ne 0) { $currentBranch = '未知' }
    Write-Host ''
    Write-Host '将要执行的操作：'
    Write-Host "  tag  ：$Tag"
    if ($Beta) {
        Write-Host '  类型 ：beta 测试版（GitHub Actions 打包并以 Prerelease 发布）'
    }
    else {
        Write-Host '  类型 ：正式版（触发 GitHub Actions 打包并发布 Release）'
    }
    Write-Host "  分支 ：$currentBranch"
    if ($NoPush) {
        Write-Host '  动作 ：仅创建本地 tag（不推送）'
    }
    else {
        Write-Host '  动作 ：创建 tag 并推送到 origin'
    }
    $answer = Read-Host '确认执行？输入 y 继续，其他任意键取消'
    if ($answer -ne 'y' -and $answer -ne 'Y') {
        Write-Host '已取消，未做任何改动。'
        return
    }

    # 5. 创建带注释的 tag
    git tag -a $Tag -m "Release $Tag"
    if ($LASTEXITCODE -ne 0) { throw "创建 tag $Tag 失败" }
    Write-Host "已创建 tag：$Tag"

    # 6. 推送 tag 触发 Release 工作流
    if ($NoPush) {
        Write-Host "未推送（-NoPush）。如需触发 Release，请执行：git push origin $Tag"
    }
    else {
        git push origin $Tag
        if ($LASTEXITCODE -ne 0) { throw "推送 tag $Tag 失败" }
        Write-Host "已推送 tag $Tag，GitHub Actions 将自动打包并发布 Release。"
        if ($Beta) {
            Write-Host 'beta 版将作为 Prerelease 发布。'
        }
    }
}
finally {
    Pop-Location
}