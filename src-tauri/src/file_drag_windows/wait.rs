//! 把网络请求交给 Tokio；COM 同步回调等待期间仍分派 STA 消息。
use super::transfer::Transfer;
use std::{
    future::Future,
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    sync::{Arc, atomic::Ordering, mpsc},
};
use terminal_core::{Result, SessionError};
use terminal_files::file_error;
use windows::{
    Win32::{
        Foundation::{HANDLE, RPC_S_CALLPENDING},
        System::{
            Com::{
                COWAIT_DISPATCH_CALLS, COWAIT_DISPATCH_WINDOW_MESSAGES, CoWaitForMultipleHandles,
            },
            Threading::{CreateEventW, SetEvent},
        },
    },
    core::PCWSTR,
};

struct Waiting(Arc<Transfer>);
impl Drop for Waiting {
    fn drop(&mut self) {
        self.0.waiting.fetch_sub(1, Ordering::AcqRel);
    }
}

pub(super) fn wait<T: Send + 'static>(
    transfer: &Arc<Transfer>,
    future: impl Future<Output = Result<T>> + Send + 'static,
) -> Result<T> {
    if transfer.cancelled() {
        return Err(SessionError::cancelled());
    }
    if transfer.waiting.fetch_add(1, Ordering::AcqRel) >= 128 {
        transfer.waiting.fetch_sub(1, Ordering::AcqRel);
        return Err(file_error("原生文件请求重入过多，传输已停止"));
    }
    let _waiting = Waiting(transfer.clone());
    let event = unsafe { CreateEventW(None, true, false, PCWSTR::null()) }
        .map_err(|_| file_error("无法创建文件传输通知"))?;
    // OwnedHandle 提供内核事件跨线程共享和唯一关闭，不给 COM 接口添加 Send。
    let event = Arc::new(unsafe { OwnedHandle::from_raw_handle(event.0) });
    let signal = event.clone();
    let (sender, receiver) = mpsc::sync_channel(1);
    transfer.runtime.spawn(async move {
        let result = future.await;
        let _ = sender.send(result);
        unsafe {
            let _ = SetEvent(HANDLE(signal.as_raw_handle()));
        }
    });
    loop {
        match receiver.try_recv() {
            Ok(result) => return result,
            Err(mpsc::TryRecvError::Disconnected) => {
                return Err(file_error("文件传输请求意外结束"));
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }
        if transfer.cancelled() {
            return Err(SessionError::cancelled());
        }
        // STA 继续处理窗口/COM 调用；重入同一个流时由流的 try_lock 拒绝，避免死锁。
        let result = unsafe {
            CoWaitForMultipleHandles(
                (COWAIT_DISPATCH_CALLS.0 | COWAIT_DISPATCH_WINDOW_MESSAGES.0) as u32,
                50,
                &[HANDLE(event.as_raw_handle())],
            )
        };
        if let Err(error) = result {
            if error.code() != RPC_S_CALLPENDING {
                transfer.cancel.cancel();
                return Err(file_error("等待原生文件传输失败"));
            }
        }
    }
}
