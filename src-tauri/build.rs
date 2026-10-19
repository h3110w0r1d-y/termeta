#[path = "src/command_names.rs"]
mod command_names;

fn main() {
    // Tauri 运行时取 ICO 首个图层作为窗口图标，首层必须保持高分辨率。
    println!("cargo:rerun-if-changed=icons/icon.ico");
    println!("cargo:rerun-if-env-changed=TERMETA_API_BASE_URL");
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(command_names::COMMANDS)),
    )
    .expect("无法生成桌面应用权限清单");
}
