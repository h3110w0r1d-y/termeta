use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tauri::{Emitter, Manager};

const DROP_TTL: Duration = Duration::from_secs(300);
const MAX_DROPS: usize = 16;
const MAX_DROP_PATHS: usize = 1000;

struct LocalDrop {
    owner: String,
    paths: Vec<PathBuf>,
    created: Instant,
}

/// 只接受操作系统拖放事件登记的路径；前端不能自行声明本地文件授权。
#[derive(Default)]
pub struct LocalDropRegistry {
    drops: Mutex<HashMap<String, LocalDrop>>,
    outgoing: Mutex<Weak<OutgoingDrag>>,
    external_entries: Mutex<HashSet<String>>,
}

/// 原生内部落点可直接终止拖出，不依赖AppKit源结束回调或对象析构。
#[derive(Default)]
pub struct OutgoingDrag {
    cancel: terminal_core::CancellationToken,
    native_started: AtomicBool,
}
impl OutgoingDrag {
    pub async fn cancelled(&self) {
        self.cancel.cancelled().await;
    }
    pub fn finish(&self) {
        self.cancel.cancel();
    }
    pub fn mark_native_started(&self) {
        self.native_started.store(true, Ordering::Release);
    }
    fn needs_mouse_release(&self) -> bool {
        self.native_started.load(Ordering::Acquire)
    }
}

impl LocalDropRegistry {
    /// 命令独占本次手势，完成松键收尾前不允许另一命令复用或替换令牌。
    pub fn begin_outgoing(&self) -> Result<Arc<OutgoingDrag>, String> {
        let mut active = self.outgoing.lock().unwrap();
        if active.upgrade().is_some() {
            return Err("原生拖动正在进行，请稍后重试".into());
        }
        let token = Arc::new(OutgoingDrag::default());
        *active = Arc::downgrade(&token);
        Ok(token)
    }

    fn register(&self, owner: &str, paths: &[PathBuf]) -> Option<String> {
        if paths.is_empty() || paths.len() > MAX_DROP_PATHS {
            return None;
        }
        let mut drops = self.drops.lock().unwrap();
        drops.retain(|_, drop| drop.created.elapsed() < DROP_TTL);
        if drops.len() >= MAX_DROPS {
            // 无关区域的拖入可能无人消费，淘汰最旧授权避免阻塞后续拖放。
            if let Some(oldest) = drops
                .iter()
                .min_by_key(|(_, drop)| drop.created)
                .map(|(id, _)| id.clone())
            {
                drops.remove(&oldest);
            }
        }
        let id = uuid::Uuid::new_v4().to_string();
        drops.insert(
            id.clone(),
            LocalDrop {
                owner: owner.to_owned(),
                paths: paths.to_vec(),
                created: Instant::now(),
            },
        );
        Some(id)
    }

    pub fn take(&self, owner: &str, id: &str) -> Result<Vec<PathBuf>, String> {
        let mut drops = self.drops.lock().unwrap();
        drops.retain(|_, drop| drop.created.elapsed() < DROP_TTL);
        if !drops.get(id).is_some_and(|drop| drop.owner == owner) {
            return Err("拖入授权已过期或不属于当前窗口，请重新拖入".into());
        }
        Ok(drops.remove(id).unwrap().paths)
    }

    pub fn clear_owner(&self, owner: &str) {
        self.drops
            .lock()
            .unwrap()
            .retain(|_, drop| drop.owner != owner);
        self.external_entries.lock().unwrap().remove(owner);
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DropPayload {
    drop_id: String,
    position: tauri::LogicalPosition<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Clone, Serialize)]
struct HoverPayload {
    position: tauri::LogicalPosition<f64>,
}

// 当前Wry在macOS/GTK返回逻辑坐标，Windows返回像素；统一为前端逻辑坐标。
fn drop_position(
    window: &tauri::Window,
    position: tauri::PhysicalPosition<f64>,
) -> tauri::LogicalPosition<f64> {
    #[cfg(not(target_os = "windows"))]
    let scale = {
        let _ = window;
        1.0
    };
    #[cfg(target_os = "windows")]
    let scale = window.scale_factor().unwrap_or(1.0);
    tauri::LogicalPosition::new(position.x / scale, position.y / scale)
}

pub fn on_window_event(window: &tauri::Window, event: &tauri::WindowEvent) {
    let Some(registry) = window.try_state::<LocalDropRegistry>() else {
        return;
    };
    // 只有受信任的主窗口参与文件授权；其他窗口不广播本地路径或引用。
    if window.label() != "main" {
        return;
    }
    if let tauri::WindowEvent::DragDrop(event) = event {
        let outgoing = registry.outgoing.lock().unwrap().upgrade();
        if let Some(outgoing) = outgoing.filter(|token| !token.cancel.is_cancelled()) {
            registry
                .external_entries
                .lock()
                .unwrap()
                .remove(window.label());
            if matches!(event, tauri::DragDropEvent::Drop { .. }) {
                // 软件内松开的Drop是明确取消信号，即使AppKit回报Copy也不等待兑现。
                outgoing.finish();
            }
            let _ = window.emit("sftp-local-leave", ());
            return;
        }
    }
    match event {
        tauri::WindowEvent::DragDrop(tauri::DragDropEvent::Enter {
            paths, position, ..
        }) => {
            registry
                .external_entries
                .lock()
                .unwrap()
                .remove(window.label());
            // 文件承诺等无本地路径的拖动不是上传源；必须先有真实外部Enter授权。
            if paths.is_empty() {
                return;
            }
            registry
                .external_entries
                .lock()
                .unwrap()
                .insert(window.label().into());
            let _ = window.emit(
                "sftp-local-hover",
                HoverPayload {
                    position: drop_position(window, *position),
                },
            );
        }
        tauri::WindowEvent::DragDrop(tauri::DragDropEvent::Over { position }) => {
            if registry
                .external_entries
                .lock()
                .unwrap()
                .contains(window.label())
            {
                let _ = window.emit(
                    "sftp-local-hover",
                    HoverPayload {
                        position: drop_position(window, *position),
                    },
                );
            }
        }
        tauri::WindowEvent::DragDrop(tauri::DragDropEvent::Drop { paths, position }) => {
            let external = registry
                .external_entries
                .lock()
                .unwrap()
                .remove(window.label());
            if !external || paths.is_empty() {
                return;
            }
            if let Some(drop_id) = registry.register(window.label(), paths) {
                let _ = window.emit(
                    "sftp-local-drop",
                    DropPayload {
                        drop_id,
                        position: drop_position(window, *position),
                        error: None,
                    },
                );
            } else {
                let _ = window.emit(
                    "sftp-local-drop",
                    DropPayload {
                        drop_id: String::new(),
                        position: drop_position(window, *position),
                        error: Some("拖入文件超过1000个根路径，请分批拖入".into()),
                    },
                );
            }
        }
        tauri::WindowEvent::DragDrop(tauri::DragDropEvent::Leave) => {
            registry
                .external_entries
                .lock()
                .unwrap()
                .remove(window.label());
            let _ = window.emit("sftp-local-leave", ());
        }
        tauri::WindowEvent::Destroyed => registry.clear_owner(window.label()),
        _ => {}
    }
}

pub struct PromisedFile {
    pub name: String,
    pub directory: bool,
    /// 目标来自原生文件承诺，回调在后台队列执行，不阻塞主线程。
    pub write: Arc<dyn Fn(PathBuf) -> Result<(), String> + Send + Sync>,
}

#[cfg(target_os = "macos")]
#[path = "file_drag_macos.rs"]
mod macos;

pub async fn start_drag(
    window: tauri::WebviewWindow,
    files: Vec<PromisedFile>,
    outgoing: Arc<OutgoingDrag>,
) -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    {
        macos::start_drag(window, files, outgoing).await
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (window, files, outgoing);
        Err("当前平台尚不支持原生文件拖出，请使用下载按钮选择目录".into())
    }
}

/// 原生手势截获松键后，先平衡WebView的鼠标序列，再恢复前端交互。
pub async fn finish_drag(
    window: &tauri::WebviewWindow,
    outgoing: &OutgoingDrag,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    if outgoing.needs_mouse_release() {
        return macos::finish_drag(window).await;
    }
    let _ = (window, outgoing);
    Ok(())
}
