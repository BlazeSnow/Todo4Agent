<#
.SYNOPSIS
  根据 package.json 中的版本号创建并推送 git tag，触发 GitHub Actions 自动发布 Release。

.DESCRIPTION
  仓库的 Release 工作流在推送 v* 格式的 tag 时自动打包并创建 Release（含 -beta 后缀的
  tag 会标为 Prerelease）。
  本脚本默认读取仓库根目录 package.json 的 version 字段作为发版版本（纯 X.Y.Z，不含
  后缀）；也可以用 -Tag 显式指定。
  运行时会先询问是否发布正式版：输入 y 打正式版 tag（vX.Y.Z），输入其他任意内容打 beta
  测试版 tag（vX.Y.Z-beta.N，序号自动递增）。
  执行前会打印将要执行的操作并要求输入 y 确认，输入其他内容则直接取消、不做任何改动。

.PARAMETER Tag
  要打的版本号，格式 vX.Y.Z，例如 v1.5.0。缺省时从 package.json 的 version 字段读取。
  正式版直接使用该版本号；测试版在其后追加 -beta.N。

.PARAMETER NoPush
  只创建 tag，不推送到远程（不会触发任何工作流）。

.EXAMPLE
  .\tag.ps1                       # 询问后：正式版 v1.4.4 或测试版 v1.4.4-beta.N
  .\tag.ps1 -Tag v1.5.0           # 指定版本号
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
    # 1. 确定版本号：优先用 -Tag，否则从 package.json 的 version 字段读取
    if (-not $Tag) {
        $pkg = Get-Content (Join-Path $RepoRoot 'package.json') -Raw -Encoding UTF8 | ConvertFrom-Json
        $version = $pkg.version
        if (-not $version) {
            throw 'package.json 中缺少 version 字段，请先填写或用 -Tag 显式指定。'
        }
        # 只取 X.Y.Z 部分，忽略 -beta 等预发布后缀（发版序号由下面正式版/beta 流程决定）
        if ($version -notmatch '^\d+\.\d+\.\d+') {
            throw "package.json 的 version 格式不正确：$version（应为 X.Y.Z，例如 1.5.0）"
        }
        $Tag = 'v' + $Matches[0]
        Write-Host "从 package.json 读取到版本：$version，本次以 $Tag 为基准"
    }

    # 2. 校验格式
    if ($Tag -notmatch '^v\d+\.\d+\.\d+$') {
        throw "tag 格式不正确：$Tag（应为 vX.Y.Z，例如 v1.5.0）"
    }

    # 3. 询问正式版还是测试版：输入 y 为正式版，其他任意内容为 beta 测试版
    $answer = Read-Host '是否发布正式版？输入 y 为正式版，其他任意键为 beta 测试版'
    $Beta = $answer -ne 'y' -and $answer -ne 'Y'
    if ($Beta) {
        $pattern = "$Tag-beta."
        $nums = @()
        git tag --list "$pattern*" | ForEach-Object {
            if ($_ -match "$pattern(\d+)$") { $nums += [int]$Matches[1] }
        }
        if ($LASTEXITCODE -ne 0) { throw 'git tag --list 执行失败' }

        git ls-remote --tags origin "$pattern*" | ForEach-Object {
            if ($_ -match "$pattern(\d+)$") { $nums += [int]$Matches[1] }
        }
        if ($LASTEXITCODE -ne 0) { throw 'git ls-remote 执行失败' }

        # 本地和远程可能同时存在同一 tag（如 v1.4.4-beta.1），去重后再计算序号
        $existing = $nums | Sort-Object -Unique
        $next = if ($existing.Count -eq 0) { 1 } else { ($existing | Measure-Object -Maximum).Maximum + 1 }
        $Tag = "$Tag-beta.$next"
        if ($existing.Count -eq 0) {
            Write-Host "未找到已存在的 beta tag，本次为 $Tag"
        }
        else {
            Write-Host "已存在 beta 序号：$($existing -join ', ')，本次为 $Tag"
        }
    }

    # 4. 检查本地和远程是否已存在该 tag
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

    # 5. 打印将要执行的操作，要求输入 y 确认
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

    # 6. 创建带注释的 tag
    git tag -a $Tag -m "Release $Tag"
    if ($LASTEXITCODE -ne 0) { throw "创建 tag $Tag 失败" }
    Write-Host "已创建 tag：$Tag"

    # 7. 推送 tag 触发 Release 工作流
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