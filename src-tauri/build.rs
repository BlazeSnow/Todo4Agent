use std::path::Path;

fn main() {
    // 前端尚未构建（dist 不存在）时生成占位页面，保证 rust-embed 编译期嵌入不失败
    let dist = Path::new("../dist");
    if !dist.exists() {
        let _ = std::fs::create_dir_all(dist);
        let _ = std::fs::write(
            dist.join("index.html"),
            "<!DOCTYPE html><html lang=\"zh-CN\"><head><meta charset=\"UTF-8\"><title>Todo4Agent</title></head><body style=\"font-family:system-ui;display:grid;place-items:center;height:100vh;margin:0\"><p>前端尚未构建：请先在仓库根目录执行 <code>pnpm build</code> 后重新编译。</p></body></html>",
        );
    }
    tauri_build::build()
}