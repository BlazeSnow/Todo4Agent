<#
.SYNOPSIS
  将 VERSION 文件中的版本号同步到 package.json、tauri.conf.json 与 src-tauri/Cargo.toml、Cargo.lock。

.DESCRIPTION
  版本号只需维护在仓库根目录的 VERSION 文件中（格式 X.Y.Z 或 X.Y.Z-beta.N，如 1.0.0、1.0.0-beta.2）。
  本脚本会写入：
    - package.json 的 version
    - tauri.conf.json 的 version（应用版本）
    - tauri.conf.json 的 bundle.windows.wix.version（MSI 数字版本：
      beta 序号映射到第 4 段，如 1.0.0-beta.1 -> 1.0.0.1；正式版为 1.0.0）
    - src-tauri/Cargo.toml 的 [package] version（Rust 后端版本）
    - src-tauri/Cargo.lock 中根包 todo4agent 的 version（保持锁文件一致）
  之后可运行 tag.ps1 按同一版本打 tag。

.EXAMPLE
  .\version.ps1        # 按 VERSION 文件同步五个版本位置
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

# Cargo.toml 的 [package] version 与 Cargo.lock 中根包 todo4agent 的 version。
# Cargo.lock 中其他包的 version 行不能动，因此按「name 行 + 紧跟的 version 行」定位。
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)
$cargoFiles = @(
    (Join-Path $RepoRoot 'src-tauri\Cargo.toml'),
    (Join-Path $RepoRoot 'src-tauri\Cargo.lock')
)
foreach ($path in $cargoFiles) {
    if (-not (Test-Path $path)) { throw "缺少文件：$path" }
    $content = [System.IO.File]::ReadAllText($path)
    $pattern = if ($path -like '*Cargo.toml') {
        '(?m)(^version = ")[^"]*"'
    } else {
        '(?m)(name = "todo4agent"\r?\nversion = ")[^"]*"'
    }
    if (-not [regex]::IsMatch($content, $pattern)) {
        throw "未找到待同步的版本行：$path"
    }
    $new = [regex]::Replace($content, $pattern, ('${1}' + $version + '"'))
    [System.IO.File]::WriteAllText($path, $new, $utf8NoBom)
}

Write-Host "已同步 package.json / tauri.conf.json / Cargo.toml / Cargo.lock（version=$version，wix.version=$wixVersion）"