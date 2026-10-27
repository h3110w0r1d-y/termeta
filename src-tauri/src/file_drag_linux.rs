use super::{LocalDropRegistry, OutgoingDrag, PromisedFile};
use gtk::{gdk, glib, prelude::*};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    os::unix::fs::DirBuilderExt,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tauri::Manager;
use tokio::sync::{Notify, oneshot};

type DragResult = Result<bool, String>;
const EXPORT_TTL: Duration = Duration::from_secs(24 * 60 * 60);
const MAX_EXPORTS: usize = 16;

/// URI 接收端可能在 drag-end 后才读取，成功批次保留一天或至正常退出。
/// 只清理本进程独占创建的精确目录，不扫描或删除其他进程的临时文件。
#[derive(Default)]
pub(super) struct Exports {
    roots: Mutex<HashMap<PathBuf, Arc<Stage>>>,
}
impl Exports {
    fn create(&self) -> Result<Arc<Stage>, String> {
        if self.roots.lock().unwrap().len() >= MAX_EXPORTS {
            return Err("拖出暂存批次已达16批，请稍后重试或使用下载按钮".into());
        }
        let temporary = std::fs::canonicalize(std::env::temp_dir())
            .map_err(|error| format!("无法访问本地临时目录：{error}"))?;
        let root = temporary.join(format!("termeta-sftp-{}", uuid::Uuid::new_v4()));
        // 原子创建和 0700 权限阻止其他用户读取，也不复用已有目录或链接。
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&root)
            .map_err(|error| format!("无法创建拖出暂存目录：{error}"))?;
        let stage = Arc::new(Stage {
            root: root.clone(),
            cleaned: Mutex::new(false),
        });
        self.roots.lock().unwrap().insert(root, stage.clone());
        Ok(stage)
    }

    pub(super) fn contains(&self, paths: &[PathBuf]) -> bool {
        let roots = self.roots.lock().unwrap();
        paths
            .iter()
            .any(|path| roots.keys().any(|root| path.starts_with(root)))
    }

    fn remove(&self, root: &Path) -> Result<(), String> {
        // 不持有登记锁执行文件系统 I/O；失败保留登记以拒绝回流并限制残留批次。
        let stage = self.roots.lock().unwrap().get(root).cloned();
        if let Some(stage) = stage {
            stage.cleanup()?;
            self.roots.lock().unwrap().remove(root);
        }
        Ok(())
    }

    pub(super) fn clear(&self) {
        // 定时清理任务仍持有 Arc；正常退出必须显式清理，不能只释放登记引用。
        let roots: Vec<_> = self.roots.lock().unwrap().keys().cloned().collect();
        for root in roots {
            let _ = self.remove(&root);
        }
    }
}
struct Stage {
    root: PathBuf,
    cleaned: Mutex<bool>,
}
impl Stage {
    fn cleanup(&self) -> Result<(), String> {
        let mut cleaned = self.cleaned.lock().unwrap();
        if !*cleaned {
            match std::fs::remove_dir_all(&self.root) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(format!(
                        "拖出暂存目录清理失败：{}：{error}",
                        self.root.display()
                    ));
                }
            }
            *cleaned = true;
        }
        Ok(())
    }
}
impl Drop for Stage {
    fn drop(&mut self) {
        // 显式清理后不再按路径重删，避免误删后来在相同名称下创建的目录。
        let _ = self.cleanup();
    }
}

#[derive(Default)]
struct Worker {
    running: AtomicBool,
    done: Notify,
}
impl Worker {
    async fn drained(&self) {
        loop {
            let changed = self.done.notified();
            if !self.running.load(Ordering::Acquire) {
                return;
            }
            changed.await;
        }
    }
}
struct WorkerGuard(Arc<Worker>);
impl Drop for WorkerGuard {
    fn drop(&mut self) {
        self.0.running.store(false, Ordering::Release);
        self.0.done.notify_one();
    }
}

struct Source {
    app: tauri::AppHandle,
    outgoing: Arc<OutgoingDrag>,
    files: Arc<Vec<PromisedFile>>,
    sender: RefCell<Option<oneshot::Sender<DragResult>>>,
    context: RefCell<Option<gdk::DragContext>>,
    stage: RefCell<Option<Arc<Stage>>>,
    uris: RefCell<Option<Vec<String>>>,
    worker: Arc<Worker>,
    loading: Cell<bool>,
    ended: Cell<bool>,
    delivered: Cell<bool>,
    accepted: Cell<bool>,
    failed: Cell<bool>,
}
impl Source {
    fn live(&self) -> bool {
        !self.outgoing.is_cancelled()
            && !self.ended.get()
            && self
                .sender
                .borrow()
                .as_ref()
                .is_some_and(|sender| !sender.is_closed())
    }

    fn complete(&self, result: DragResult) {
        if let Some(sender) = self.sender.borrow_mut().take() {
            let _ = sender.send(result);
        }
    }

    fn owns(&self, context: &gdk::DragContext) -> bool {
        self.context.borrow().as_ref() == Some(context)
    }

    fn provide(&self, context: &gdk::DragContext, selection: &gtk::SelectionData) {
        if !self.owns(context) || !self.live() || self.loading.get() {
            return;
        }
        // GTK 的 OTHER_APP 约束之外，再拒绝本进程的原生窗口；不能将内部落点变成上传。
        if internal_destination(context) {
            self.outgoing.finish();
            return;
        }
        if self.uris.borrow().is_none() {
            self.loading.set(true);
            let result = self.prepare();
            self.loading.set(false);
            match result {
                Ok(uris) if self.live() => *self.uris.borrow_mut() = Some(uris),
                Ok(_) => return,
                Err(error) => {
                    self.complete(Err(error));
                    return;
                }
            }
        }
        if !self.live() || internal_destination(context) {
            return;
        }
        if let Some(uris) = self.uris.borrow().as_ref() {
            if selection.set_uris(&uris.iter().map(String::as_str).collect::<Vec<_>>()) {
                self.delivered.set(true);
            } else {
                self.complete(Err("无法交付拖出文件列表".into()));
            }
        }
    }

    fn prepare(&self) -> Result<Vec<String>, String> {
        let stage = self.app.state::<LocalDropRegistry>().exports.create()?;
        *self.stage.borrow_mut() = Some(stage.clone());
        let files = self.files.clone();
        let outgoing = self.outgoing.clone();
        let worker = self.worker.clone();
        worker.running.store(true, Ordering::Release);
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        tauri::async_runtime::spawn_blocking(move || {
            let _worker = WorkerGuard(worker);
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let mut uris = Vec::with_capacity(files.len());
                for file in files.iter() {
                    if outgoing.is_cancelled() {
                        return Err("拖出下载已取消".into());
                    }
                    // 名称已经过 child_path 校验；本地路径仅由 Rust 生成，不来自 WebView。
                    let local = stage.root.join(&file.name);
                    (file.write)(local.clone())?;
                    uris.push(
                        url::Url::from_file_path(local)
                            .map_err(|_| "无法生成拖出文件 URI".to_owned())?
                            .to_string(),
                    );
                }
                Ok(uris)
            }))
            .unwrap_or_else(|_| Err("原生拖出下载回调异常".into()));
            let _ = sender.send(result);
        });
        // GTK3 的 selection 回调要求同步填充；嵌套主循环只泵送 GTK 消息，网络和磁盘仍在后台。
        // 拒绝 provide 重入；取消后继续等后台退出，避免删除仍在写入的暂存目录。
        let waiting = glib::MainLoop::new(None, false);
        let completed = Rc::new(RefCell::new(None));
        let result = completed.clone();
        let wake = waiting.clone();
        glib::timeout_add_local(Duration::from_millis(20), move || {
            match receiver.try_recv() {
                Ok(value) => *result.borrow_mut() = Some(value),
                Err(std::sync::mpsc::TryRecvError::Empty) => return glib::ControlFlow::Continue,
                Err(_) => *result.borrow_mut() = Some(Err("拖出下载后台已结束".into())),
            }
            wake.quit();
            glib::ControlFlow::Break
        });
        waiting.run();
        completed
            .borrow_mut()
            .take()
            .unwrap_or_else(|| Err("拖出下载已中断".into()))
    }
}

/// GDK 的 dest_window 允许 NULL，生成的 Rust getter 不允许，故在此保留 Option。
fn internal_destination(context: &gdk::DragContext) -> bool {
    use glib::translate::{ToGlibPtr, from_glib_none};
    let destination: Option<gdk::Window> = unsafe {
        from_glib_none(gdk::ffi::gdk_drag_context_get_dest_window(
            context.to_glib_none().0,
        ))
    };
    destination.is_some_and(|destination| {
        gtk::Window::list_toplevels().iter().any(|window| {
            window
                .window()
                .is_some_and(|local| local.effective_toplevel() == destination.effective_toplevel())
        })
    })
}

struct Active {
    widget: gtk::ApplicationWindow,
    source: Rc<Source>,
    signals: Vec<glib::SignalHandlerId>,
    watch: glib::SourceId,
}
thread_local! {
    static ACTIVE: RefCell<Option<Active>> = const { RefCell::new(None) };
}

pub(super) async fn start_drag(
    window: tauri::WebviewWindow,
    files: Vec<PromisedFile>,
    outgoing: Arc<OutgoingDrag>,
) -> DragResult {
    let (sender, receiver) = oneshot::channel();
    let native = window.clone();
    window
        .run_on_main_thread(move || {
            if sender.is_closed() || outgoing.is_cancelled() {
                return;
            }
            let source = Rc::new(Source {
                app: native.app_handle().clone(),
                outgoing,
                files: Arc::new(files),
                sender: RefCell::new(Some(sender)),
                context: RefCell::new(None),
                stage: RefCell::new(None),
                uris: RefCell::new(None),
                worker: Arc::new(Worker::default()),
                loading: Cell::new(false),
                ended: Cell::new(false),
                delivered: Cell::new(false),
                accepted: Cell::new(false),
                failed: Cell::new(false),
            });
            if let Err(error) = begin(&native, source.clone()) {
                source.complete(Err(error));
            }
        })
        .map_err(|error| error.to_string())?;
    receiver
        .await
        .map_err(|_| "原生拖出会话已中断".to_owned())?
}

fn begin(window: &tauri::WebviewWindow, source: Rc<Source>) -> Result<(), String> {
    if ACTIVE.with(|active| active.borrow().is_some()) {
        return Err("原生拖出正在收尾，请稍后重试".into());
    }
    let widget = window.gtk_window().map_err(|error| error.to_string())?;
    let native = widget.window().ok_or("原生窗口不可用")?;
    let device = widget
        .display()
        .default_seat()
        .and_then(|seat| seat.pointer())
        .ok_or("当前桌面没有可用的鼠标设备")?;
    let (_, x, y, buttons) = native.device_position(&device);
    if !buttons.contains(gdk::ModifierType::BUTTON1_MASK) {
        source.complete(Ok(false));
        return Ok(());
    }
    // IPC 到达时 current_event 可能已过期，使用当前设备和仍按住的真实按键重建事件。
    let mut event = gdk::Event::new(gdk::EventType::MotionNotify);
    {
        use glib::translate::ToGlibPtr;
        let motion = event
            .downcast_mut::<gdk::EventMotion>()
            .ok_or("无法创建拖动事件")?;
        motion.as_mut().window = native.to_glib_full();
        motion.as_mut().time = gdk::CURRENT_TIME;
        motion.as_mut().x = f64::from(x);
        motion.as_mut().y = f64::from(y);
        motion.as_mut().state = buttons.bits();
    }
    event.set_device(Some(&device));
    let provide = source.clone();
    let get =
        widget.connect_drag_data_get(move |_, context, data, _, _| provide.provide(context, data));
    let failed = source.clone();
    let fail = widget.connect_drag_failed(move |_, context, reason| {
        if failed.owns(context) {
            failed.failed.set(true);
            failed.complete(match reason {
                gtk::DragResult::NoTarget | gtk::DragResult::UserCancelled => Ok(false),
                _ => Err(format!("原生文件拖出失败：{reason}")),
            });
        }
        glib::Propagation::Stop
    });
    let ended = source.clone();
    let end = widget.connect_drag_end(move |_, context| {
        if !ended.owns(context) {
            return;
        }
        ended.ended.set(true);
        let accepted = !ended.outgoing.is_cancelled()
            && !ended.failed.get()
            && ended.delivered.get()
            && !internal_destination(context)
            && context.selected_action() == gdk::DragAction::COPY;
        ended.accepted.set(accepted);
        ended.complete(Ok(accepted));
    });
    let targets = gtk::TargetList::new(&[gtk::TargetEntry::new(
        "text/uri-list",
        gtk::TargetFlags::OTHER_APP,
        1,
    )]);
    let context =
        widget.drag_begin_with_coordinates(&targets, gdk::DragAction::COPY, 1, Some(&event), x, y);
    let Some(context) = context else {
        for signal in [get, fail, end] {
            widget.disconnect(signal);
        }
        return Err("无法启动 GTK 原生文件拖出".into());
    };
    source.outgoing.mark_native_started();
    *source.context.borrow_mut() = Some(context.clone());
    context.drag_set_icon_name("text-x-generic", 0, 0);
    let watching = source.clone();
    let cancel_context = context.clone();
    let watch = glib::timeout_add_local(Duration::from_millis(50), move || {
        if !watching.live() {
            if !watching.ended.replace(true) {
                cancel_context.drag_cancel();
            }
            return glib::ControlFlow::Break;
        }
        glib::ControlFlow::Continue
    });
    ACTIVE.with(|active| {
        *active.borrow_mut() = Some(Active {
            widget,
            source,
            signals: vec![get, fail, end],
            watch,
        })
    });
    Ok(())
}

pub(super) async fn finish_drag(window: &tauri::WebviewWindow) -> Result<(), String> {
    let (sender, receiver) = oneshot::channel();
    window
        .run_on_main_thread(move || {
            let active = ACTIVE.with(|active| active.borrow_mut().take());
            let cleanup = active.map(|active| {
                // 先撤销所有回调再取消 GTK；回调中不借用 ACTIVE，允许嵌套主循环安全收尾。
                for signal in active.signals {
                    active.widget.disconnect(signal);
                }
                // watch 可能已自行移除；查找仍存活的 Source，避免重复 remove 触发 GLib 错误。
                if let Some(watch) = glib::MainContext::default().find_source_by_id(&active.watch) {
                    watch.destroy();
                }
                if !active.source.ended.replace(true) {
                    if let Some(context) = active.source.context.borrow().as_ref() {
                        context.drag_cancel();
                    }
                }
                let stage = active.source.stage.borrow().clone();
                (
                    active.source.worker.clone(),
                    stage,
                    active.source.accepted.get(),
                )
            });
            let _ = sender.send(cleanup);
        })
        .map_err(|error| error.to_string())?;
    let cleanup = receiver
        .await
        .map_err(|_| "原生拖出清理已中断".to_owned())?;
    if let Some((worker, stage, accepted)) = cleanup {
        worker.drained().await;
        let mouse_released = release_webview_mouse(window).await;
        if let Some(stage) = stage {
            if accepted {
                let app = window.app_handle().clone();
                // 接收端的复制不受 SFTP 会话寿命约束；只有成功交付的暂存批次保留宽限期。
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(EXPORT_TTL).await;
                    let _ = tauri::async_runtime::spawn_blocking(move || {
                        app.state::<LocalDropRegistry>().exports.remove(&stage.root)
                    })
                    .await;
                });
            } else {
                let app = window.app_handle().clone();
                tauri::async_runtime::spawn_blocking(move || {
                    app.state::<LocalDropRegistry>().exports.remove(&stage.root)
                })
                .await
                .map_err(|error| error.to_string())??;
            }
        }
        mouse_released?;
    }
    Ok(())
}

/// GTK 接管了真实松键时，补齐 WebKit 的按键序列；不改变焦点，也不伪造仍按住的松键。
async fn release_webview_mouse(window: &tauri::WebviewWindow) -> Result<(), String> {
    let (sender, receiver) = oneshot::channel();
    window
        .with_webview(move |webview| {
            let view = webview.inner();
            if let Some(native) = view.window() {
                if let Some(device) = view
                    .display()
                    .default_seat()
                    .and_then(|seat| seat.pointer())
                {
                    let (_, x, y, buttons) = native.device_position(&device);
                    if !buttons.contains(gdk::ModifierType::BUTTON1_MASK) {
                        use glib::translate::ToGlibPtr;
                        let mut event = gdk::Event::new(gdk::EventType::ButtonRelease);
                        if let Some(button) = event.downcast_mut::<gdk::EventButton>() {
                            button.as_mut().window = native.to_glib_full();
                            button.as_mut().time = gdk::CURRENT_TIME;
                            button.as_mut().button = 1;
                            button.as_mut().x = f64::from(x);
                            button.as_mut().y = f64::from(y);
                            button.as_mut().state = buttons.bits();
                        }
                        event.set_device(Some(&device));
                        view.event(&event);
                    }
                }
            }
            let _ = sender.send(());
        })
        .map_err(|error| error.to_string())?;
    receiver.await.map_err(|_| "拖出鼠标清理已中断".into())
}
