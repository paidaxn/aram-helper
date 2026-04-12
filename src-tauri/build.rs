fn main() {
    // Windows: 嵌入管理员权限清单，让应用启动时请求管理员权限
    // 这样才能读取 LOL 客户端的进程信息
    #[cfg(target_os = "windows")]
    embed_resource::compile("admin.rc", embed_resource::NONE);

    tauri_build::build()
}
