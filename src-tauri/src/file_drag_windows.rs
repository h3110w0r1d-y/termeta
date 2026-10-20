//! Windows Shell/OLE 原生拖出；目录清单、内容流与异步结束分别处理。
mod data_object;
mod manifest;
mod stream;
mod transfer;
mod wait;

use crate::file_drag::LocalDropRegistry;
use data_object::{DataObject, Formats};
use manifest::Root;
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};
use tauri::{Manager, WebviewWindow, ipc::Channel};
use terminal_core::{CancellationToken, Result};
use terminal_files::{TransferProgress, file_error};
use terminal_sftp::SftpBrowser;
use transfer::{Decision, Transfer};
use windows::{
    Win32::{
        Foundation::{
            DRAGDROP_S_CANCEL, DRAGDROP_S_DROP, DRAGDROP_S_USEDEFAULTCURSORS, HWND, LPARAM, POINT,
            S_OK, WPARAM,
        },
        Graphics::Gdi::ScreenToClient,
        System::{
            Com::IDataObject,
            Ole::{
                DROPEFFECT, DROPEFFECT_COPY, DROPEFFECT_NONE, DoDragDrop, IDropSource,
                IDropSource_Impl, OleInitialize, OleUninitialize,
            },
            SystemServices::{MK_LBUTTON, MODIFIERKEYS_FLAGS},
        },
        UI::{
            Input::KeyboardAndMouse::{GetAsyncKeyState, GetCapture, VK_LBUTTON},
            WindowsAndMessaging::{
                GA_ROOT, GetAncestor, GetCursorPos, PostMessageW, WM_LBUTTONUP, WindowFromPoint,
            },
        },
    },
    core::{BOOL, ComObject, HRESULT, implement},
};

struct OleGuard;
impl Drop for OleGuard {
    fn drop(&mut self) {
        unsafe {
            OleUninitialize();
        }
    }
}

struct NativeDrag {
    // 字段按声明顺序析构：先释放 COM 数据对象，再平衡 OLE 初始化。
    _object: ComObject<DataObject>,
    _ole: OleGuard,
}
thread_local! {
    // STA 对象只留在主线程；不通过 unsafe Send 把 COM 接口交给 Tokio。
    static ACTIVE: RefCell<HashMap<String, NativeDrag>> = RefCell::new(HashMap::new());
}

#[implement(IDropSource)]
struct DropSource {
    transfer: Arc<Transfer>,
    owner: HWND,
}
#[allow(non_snake_case)]
impl IDropSource_Impl for DropSource_Impl {
    fn QueryContinueDrag(&self, escape: BOOL, keys: MODIFIERKEYS_FLAGS) -> HRESULT {
        if escape.as_bool() || self.transfer.cancelled() {
            return DRAGDROP_S_CANCEL;
        }
        if keys.contains(MK_LBUTTON) {
            return S_OK;
        }
        // Wry 不会把虚拟文件转成本地路径；仍需原生确认应用内松键是取消。
        let mut point = POINT::default();
        unsafe {
            if GetCursorPos(&mut point).is_err()
                || GetAncestor(WindowFromPoint(point), GA_ROOT) == self.owner
            {
                return DRAGDROP_S_CANCEL;
            }
        }
        DRAGDROP_S_DROP
    }
    fn GiveFeedback(&self, _effect: DROPEFFECT) -> HRESULT {
        DRAGDROP_S_USEDEFAULTCURSORS
    }
}

fn finish_mouse(owner: HWND, captured: HWND) {
    let target = if captured.0.is_null() {
        owner
    } else {
        captured
    };
    let mut point = POINT::default();
    unsafe {
        let _ = GetCursorPos(&mut point);
        let _ = ScreenToClient(target, &mut point);
        let position =
            LPARAM(((u32::from(point.y as u16) << 16) | u32::from(point.x as u16)) as isize);
        // 仅平衡本窗口消息序列，不伪造系统输入或触发外部目标上的鼠标动作。
        let _ = PostMessageW(Some(target), WM_LBUTTONUP, WPARAM(0), position);
    }
}

fn begin(
    window: &WebviewWindow,
    transfer: Arc<Transfer>,
    id: String,
) -> std::result::Result<(), String> {
    if transfer.cancelled() || unsafe { GetAsyncKeyState(i32::from(VK_LBUTTON.0)) } >= 0 {
        transfer.operation.lock().unwrap().drag = Some((DRAGDROP_S_CANCEL, 0));
        return Ok(());
    }
    let owner = window
        .hwnd()
        .map(|hwnd| HWND(hwnd.0))
        .map_err(|_| "无法读取拖出窗口".to_string())?;
    unsafe { OleInitialize(None) }.map_err(|_| "无法初始化 Windows 文件拖放".to_string())?;
    let ole = OleGuard;
    let formats = Formats::register().map_err(|_| "无法注册 Windows 文件拖放格式".to_string())?;
    let object = ComObject::new(DataObject {
        transfer: transfer.clone(),
        formats,
    });
    let data: IDataObject = object.to_interface();
    let source: IDropSource = DropSource {
        transfer: transfer.clone(),
        owner,
    }
    .into();
    ACTIVE.with(|active| {
        active.borrow_mut().insert(
            id,
            NativeDrag {
                _object: object,
                _ole: ole,
            },
        );
    });
    transfer.outgoing.mark_native_started();
    let captured = unsafe { GetCapture() };
    let mut effect = DROPEFFECT_NONE;
    let result = unsafe { DoDragDrop(&data, &source, DROPEFFECT_COPY, &mut effect) };
    finish_mouse(owner, captured);
    transfer.operation.lock().unwrap().drag = Some((result, effect.0));
    if result == DRAGDROP_S_DROP && effect == DROPEFFECT_COPY && !transfer.cancelled() {
        transfer.accept();
    }
    Ok(())
}

async fn outcome(transfer: &Transfer) -> Result<bool> {
    loop {
        if transfer.cancelled() {
            return Ok(false);
        }
        if let Some(error) = transfer.error.lock().unwrap().clone() {
            return Err(file_error(&error));
        }
        let decision = transfer.operation.lock().unwrap().decision();
        match decision {
            Decision::Cancelled => return Ok(false),
            Decision::Failed(message) => return Err(file_error(message)),
            Decision::Copied => {
                if !transfer.all_delivered() {
                    return Err(file_error("拖出文件未完整交付，目标可能保留部分文件"));
                }
                transfer.report(None, true, true);
                return Ok(true);
            }
            Decision::Pending => {}
        }
        // OLE 松键后主线程继续处理消息，等待目标 EndOperation，不把松键当下载完成。
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

pub(crate) async fn start_drag(
    window: WebviewWindow,
    browser: Arc<SftpBrowser>,
    session_id: String,
    names: Vec<String>,
    paths: Vec<String>,
    directories: HashSet<String>,
    cancel: CancellationToken,
    channel: Channel<TransferProgress>,
) -> Result<bool> {
    let mut keys = HashSet::new();
    let mut roots = Vec::new();
    for (name, remote) in names.into_iter().zip(paths) {
        manifest::validate_relative(&name)?;
        if !keys.insert(name.to_uppercase()) {
            return Err(file_error("Windows 无法区分选中的同名条目，请先重命名"));
        }
        roots.push(Root {
            directory: directories.contains(&name),
            name,
            remote,
        });
    }
    let outgoing = window
        .state::<LocalDropRegistry>()
        .begin_outgoing()
        .map_err(|error| file_error(&error))?;
    let transfer = Transfer::new(
        window.app_handle().clone(),
        window.label().to_owned(),
        session_id,
        browser,
        roots,
        cancel,
        outgoing,
        channel,
    );
    let id = uuid::Uuid::new_v4().to_string();
    let native_id = id.clone();
    let native_window = window.clone();
    let native_transfer = transfer.clone();
    let (sender, receiver) = tokio::sync::oneshot::channel();
    window
        .run_on_main_thread(move || {
            // 原生循环自己检查取消；Rust 命令始终等待它退出后才释放本次操作锁。
            let result = begin(&native_window, native_transfer, native_id);
            let _ = sender.send(result);
        })
        .map_err(|_| file_error("无法启动 Windows 原生拖出"))?;
    let result = match receiver.await {
        Ok(Ok(())) => outcome(&transfer).await,
        Ok(Err(error)) => Err(file_error(&error)),
        Err(_) => Err(file_error("Windows 拖出窗口已关闭")),
    };
    let closed = transfer.close().await;
    // 在创建对象的 STA 线程释放保活引用；迟到目标即使持有自己的引用，也只能得到取消。
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let scheduled = window.run_on_main_thread(move || {
        let removed = ACTIVE.with(|active| active.borrow_mut().remove(&id));
        drop(removed);
        let _ = sender.send(());
    });
    if scheduled.is_ok() {
        let _ = receiver.await;
    }
    match result {
        Ok(true) => {
            closed?;
            Ok(true)
        }
        result => result,
    }
}
