mod backup;
mod backup_runtime;
mod commands;
mod file_drag;
mod files;
#[cfg(target_os = "macos")]
mod macos_menu;
mod native_menu;
mod settings;
mod settings_window;
mod shutdown;
mod state;

use state::AppState;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tauri::Manager;
use terminal_core::SessionManager;
use terminal_security::CredentialVault;
use terminal_ssh::SshFactory;
use terminal_storage::SqliteStore;

// 发布窗口只允许应用资源导航；开发服务仅在调试配置可用。
fn local_navigation(url: &url::Url) -> bool {
    (url.scheme() == "tauri" && url.host_str() == Some("localhost") && url.port().is_none())
        || (matches!(url.scheme(), "http" | "https")
            && url.host_str() == Some("tauri.localhost")
            && url.port().is_none())
        || (cfg!(debug_assertions)
            && url.scheme() == "http"
            && url.host_str() == Some("localhost")
            && url.port() == Some(1420))
}

pub fn run() {
    tauri::Builder::default()
        .manage(shutdown::ShutdownState::default())
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(
            tauri_plugin_opener::Builder::new()
                .open_js_links_on_click(false)
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            #[cfg(target_os = "macos")]
            macos_menu::configure(app)?;
            let path = app.path().app_data_dir()?;
            std::fs::create_dir_all(&path)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))?;
            }
            // 加密格式独立代际，不读取或迁移开发阶段的旧数据库与逐项系统凭据。
            let store = Arc::new(SqliteStore::locked(
                path.join("termeta-encrypted-v2.sqlite3"),
            ));
            let vault = Arc::new(CredentialVault::new(store.clone()));
            let factory =
                SshFactory::new(store.clone(), vault.clone()).with_system_detection(store.clone());
            let ssh_channels = factory.channels();
            let file_sessions = SessionManager::new(Arc::new(factory.for_files()));
            let forward_sessions = SessionManager::new(Arc::new(factory.for_forwarding()));
            let sessions = SessionManager::new(Arc::new(factory))
                .with_factory(Arc::new(terminal_local::LocalFactory))
                .with_factory(Arc::new(terminal_serial::SerialFactory))
                .with_factory(Arc::new(terminal_telnet::TelnetFactory));
            app.manage(file_drag::LocalDropRegistry::default());
            app.manage(AppState {
                backup: backup_runtime::BackupRuntime::default(),
                settings_gate: tokio::sync::Mutex::new(()),
                sessions,
                file_sessions,
                forward_sessions,
                ssh_channels,
                sftp: Mutex::new(std::collections::HashMap::new()),
                transfers: Arc::new(tokio::sync::Semaphore::new(4)),
                store,
                vault,
                open_gates: Mutex::new(std::collections::HashMap::new()),
            });
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(30));
                loop {
                    interval.tick().await;
                    let state = handle.state::<AppState>();
                    state.backup.cleanup();
                    state.sftp.lock().unwrap().retain(|id, browser| {
                        let alive = state
                            .session_manager("main", id)
                            .and_then(|manager| manager.get("main", id))
                            .is_ok_and(|s| s.state == terminal_core::SessionState::Connected);
                        if !alive {
                            browser.cancel();
                        }
                        alive
                    });
                }
            });
            // macOS 保留原生窗口按钮，将 WebView 延伸至标题栏；其余平台由标签栏承载窗口控件。
            let window =
                tauri::WebviewWindowBuilder::from_config(app, &app.config().app.windows[0])?;
            #[cfg(target_os = "macos")]
            let window = window
                .title_bar_style(tauri::TitleBarStyle::Overlay)
                .hidden_title(true)
                // 下移原生窗口按钮，匹配 54px 标签栏的垂直居中布局。
                .traffic_light_position(tauri::LogicalPosition::new(16.0, 26.0));
            #[cfg(not(target_os = "macos"))]
            let window = window.decorations(false);
            window
                .on_navigation(local_navigation)
                .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny)
                .build()?;
            Ok(())
        })
        .on_window_event(|window, event| {
            file_drag::on_window_event(window, event);
            if matches!(event, tauri::WindowEvent::Destroyed) {
                if let Some(state) = window.try_state::<AppState>() {
                    state.backup.clear_owner(window.label());
                    state.sessions.close_owner(window.label());
                    state.file_sessions.close_owner(window.label());
                    state.forward_sessions.close_owner(window.label());
                    if window.label() == "main" {
                        #[cfg(target_os = "macos")]
                        let _ = window.run_on_main_thread(macos_menu::remove_listeners);
                        if let Some(settings) = window.app_handle().get_webview_window("settings") {
                            let _ = settings.destroy();
                        }
                        for browser in state.sftp.lock().unwrap().values() {
                            browser.cancel();
                        }
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_backup_config,
            commands::save_backup_config,
            commands::test_backup_config,
            commands::list_cloud_backups,
            commands::create_cloud_backup,
            commands::prepare_cloud_restore,
            commands::apply_cloud_restore,
            commands::initialize_from_cloud_backup,
            commands::list_port_forwards,
            commands::save_port_forward,
            commands::delete_port_forward,
            commands::start_port_forward,
            commands::stop_port_forward,
            commands::sftp_connect,
            commands::sftp_copy,
            commands::sftp_drop_upload,
            commands::sftp_start_drag,
            commands::sftp_list,
            commands::sftp_transfer,
            commands::sftp_cancel,
            commands::available_session_kinds,
            commands::list_serial_ports,
            commands::open_session,
            commands::write_session,
            commands::resize_session,
            commands::ack_session_output,
            commands::respond_host_key,
            commands::respond_auth_challenge,
            commands::cancel_auth_challenge,
            commands::close_session,
            commands::list_sessions,
            commands::get_session,
            commands::session_client_heartbeat,
            commands::vault_status,
            commands::unlock_vault,
            commands::change_vault_password,
            commands::list_hosts,
            commands::save_host,
            commands::save_workspace,
            commands::delete_host,
            commands::list_host_groups,
            commands::save_host_group,
            commands::delete_host_group,
            commands::list_known_hosts,
            commands::revoke_known_host,
            commands::select_private_key_file,
            commands::list_credentials,
            commands::save_credential,
            commands::delete_credential,
            commands::open_settings_window,
            commands::set_native_menu_locale,
            commands::get_settings,
            commands::save_settings,
            commands::open_external_url,
            commands::export_json,
            commands::list_snippets,
            commands::save_snippet,
            commands::delete_snippet,
            commands::save_session_log,
            commands::list_session_logs,
            commands::read_session_log,
            commands::delete_session_log,
            commands::clear_session_logs,
            shutdown::finish_app_exit,
        ])
        .build(tauri::generate_context!())
        .expect("Termeta 初始化失败；请检查应用数据目录与桌面环境")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                shutdown::intercept(app, api);
            }
        });
}
