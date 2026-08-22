#!/usr/bin/env bash
# CI 校验：package.json 与 src-tauri/tauri.conf.json 的版本必须与发布 tag 一致；
# 同时校验 Windows MSI 的数字版本（bundle.windows.wix.version）与版本号匹配。
# 映射规则：1.0.0-beta.1 → wix.version 1.0.0.1；正式版 1.0.0 → 1.0.0（可缺省，由 MSI 派生）
# 用法：ci/check-version.sh <期望版本>，如 bash ci/check-version.sh 1.0.0-beta.1
set -euo pipefail

want="$1"

node -e '
  const fs = require("fs");
  const want = process.argv[1];
  const pkg = JSON.parse(fs.readFileSync("package.json", "utf8"));
  const tauri = JSON.parse(fs.readFileSync("src-tauri/tauri.conf.json", "utf8"));
  const wixVersion = tauri.bundle?.windows?.wix?.version ?? null;
  const files = {
    "package.json": pkg.version,
    "src-tauri/tauri.conf.json": tauri.version,
  };
  let ok = true;
  for (const [f, v] of Object.entries(files)) {
    if (v !== want) {
      console.error(`版本不一致：${f} 为 ${v}，期望 ${want}`);
      ok = false;
    }
  }
  // wix.version 校验：beta 版本必须显式数字映射；正式版可缺省但若设置需一致
  const m = want.match(/^(\d+)\.(\d+)\.(\d+)(?:-beta\.(\d+))?$/);
  if (!m) {
    console.error(`无法解析版本：${want}`);
    process.exit(1);
  }
  const [, maj, min, pat, beta] = m;
  const expectedWix = beta ? `${maj}.${min}.${pat}.${beta}` : `${maj}.${min}.${pat}`;
  if (beta && wixVersion !== expectedWix) {
    console.error(`wix.version 应为 ${expectedWix}（MSI 数字版本），当前为 ${wixVersion ?? "未设置"}`);
    ok = false;
  }
  if (!beta && wixVersion != null && wixVersion !== expectedWix) {
    console.error(`wix.version 应为 ${expectedWix}（或删除该字段由 MSI 自动派生），当前为 ${wixVersion}`);
    ok = false;
  }
  if (!ok) process.exit(1);
  console.log(`版本一致：${want}${beta ? `，wix.version ${wixVersion}` : ""}`);
' "$want"
