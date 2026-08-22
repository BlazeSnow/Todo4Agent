<#
.SYNOPSIS
  将 VERSION 文件中的版本号同步到 package.json 与 tauri.conf.json（含 Windows MSI 的 wix.version）。

.DESCRIPTION
  版本号只需维护在仓库根目录的 VERSION 文件中（格式 X.Y.Z 或 X.Y.Z-beta.N，如 1.0.0、1.0.0-beta.2）。
  本脚本会写入：
    - package.json 的 version
    - tauri.conf.json 的 version（应用版本）
    - tauri.conf.json 的 bundle.windows.wix.version（MSI 数字版本：
      beta 序号映射到第 4 段，如 1.0.0-beta.1 -> 1.0.0.1；正式版为 1.0.0）
  之后可运行 tag.ps1 按同一版本打 tag。

.EXAMPLE
  .\version.ps1        # 按 VERSION 文件同步三个版本位置
#>

$ErrorActionPreference = 'Stop'

$RepoRoot = Split-Path -Parent $MyInvocation.MyCommand.Path

$version = (Get-Content (Join-Path $RepoRoot 'VERSION') -Raw -Encoding UTF8).Trim()
if (-not $version) {
    throw 'VERSION 文件中缺少版本号。'
}
if ($version -notmatch '^\d+\.\d+\.\d+(-beta\.\d+)?$') {
    throw "VERSION 文件格式不正确：$version（应为 X.Y.Z 或 X.Y.Z-beta.N，例如 1.0.0、1.0.0-beta.2）"
}

# wix.version：MSI 数字版本，beta 序号映射到第 4 段
$wixVersion = if ($version -match '^(.*)-beta\.(\d+)$') { "$($Matches[1]).$($Matches[2])" } else { $version }

$nodeScript = @'
const fs = require('fs');
const ver = process.argv[1], wix = process.argv[2];
const p = JSON.parse(fs.readFileSync('package.json', 'utf8'));
p.version = ver;
fs.writeFileSync('package.json', JSON.stringify(p, null, 2) + String.fromCharCode(10));
const t = JSON.parse(fs.readFileSync('src-tauri/tauri.conf.json', 'utf8'));
t.version = ver;
t.bundle.windows.wix.version = wix;
fs.writeFileSync('src-tauri/tauri.conf.json', JSON.stringify(t, null, 2) + String.fromCharCode(10));
'@
node -e $nodeScript $version $wixVersion
if ($LASTEXITCODE -ne 0) { throw '同步版本到 package.json / tauri.conf.json 失败' }

Write-Host "已同步 package.json / tauri.conf.json（version=$version，wix.version=$wixVersion）"