#!/usr/bin/env bash
# CI 校验：package.json 与 src-tauri/tauri.conf.json 的版本必须与发布 tag 一致
# 用法：ci/check-version.sh <期望版本>，如 bash ci/check-version.sh 1.0.0-beta.1
set -euo pipefail

want="$1"

node -e '
  const fs = require("fs");
  const want = process.argv[1];
  const files = {
    "package.json": JSON.parse(fs.readFileSync("package.json", "utf8")).version,
    "src-tauri/tauri.conf.json": JSON.parse(fs.readFileSync("src-tauri/tauri.conf.json", "utf8")).version,
  };
  let ok = true;
  for (const [f, v] of Object.entries(files)) {
    if (v !== want) {
      console.error(`版本不一致：${f} 为 ${v}，期望 ${want}`);
      ok = false;
    }
  }
  if (!ok) process.exit(1);
  console.log(`版本一致：${want}`);
' "$want"
