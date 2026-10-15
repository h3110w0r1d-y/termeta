use super::PromisedFile;
use block2::DynBlock;
use objc2::{
    AnyThread, DefinedClass, MainThreadOnly, define_class, msg_send, rc::Retained,
    runtime::ProtocolObject,
};
use objc2_app_kit::{
    NSApplication, NSDragOperation, NSDraggingContext, NSDraggingItem, NSDraggingSession,
    NSDraggingSource, NSEvent, NSEventModifierFlags, NSEventType, NSFilePromiseProvider,
    NSFilePromiseProviderDelegate, NSImage, NSView, NSWindow,
};
use objc2_foundation::{
    MainThreadMarker, NSArray, NSError, NSObject, NSObjectProtocol, NSOperationQueue, NSPoint,
    NSRect, NSSize, NSString, NSURL, ns_string,
};
use std::{
    cell::RefCell,
    ffi::CStr,
    os::unix::ffi::OsStrExt,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::sync::{Notify, oneshot};

type DragResult = Result<bool, String>;
struct Completion {
    ended: bool,
    remaining: usize,
    active: usize,
    requested: bool,
    changed: Arc<Notify>,
    changed_at: std::time::Instant,
    error: Option<String>,
    sender: Option<oneshot::Sender<DragResult>>,
}
impl Completion {
    fn finish(&mut self) {
        self.changed_at = std::time::Instant::now();
        self.changed.notify_one();
        if self.ended && self.remaining == 0 {
            if let Some(sender) = self.sender.take() {
                let result = self.error.take().map_or(Ok(true), Err);
                let _ = sender.send(result);
            }
        }
    }
}
struct PromiseIvars {
    file: PromisedFile,
    completion: Arc<Mutex<Completion>>,
    queue: Retained<NSOperationQueue>,
    written: AtomicBool,
}

impl Drop for PromiseIvars {
    fn drop(&mut self) {
        // Finder可能跳过冲突项；provider释放且未请求内容时，将任务收敛为失败。
        // 已开始写入的delegate由AppKit保持至回调结束，不重复扣减。
        if !self.written.swap(true, Ordering::AcqRel) {
            let mut completion = self.completion.lock().unwrap();
            completion
                .error
                .get_or_insert_with(|| "Finder未接收全部拖出文件，请检查目标目录或重试".into());
            completion.remaining = completion.remaining.saturating_sub(1);
            completion.finish();
        }
    }
}

define_class!(
    // NSObject无额外子类约束；后台回调只访问线程安全的Rust状态。
    #[unsafe(super = NSObject)]
    #[ivars = PromiseIvars]
    struct FilePromiseDelegate;
    unsafe impl NSObjectProtocol for FilePromiseDelegate {}
    unsafe impl NSFilePromiseProviderDelegate for FilePromiseDelegate {
        // 返回Objective-C对象时使用method_id，由objc2转换所有权与返回值ABI。
        #[unsafe(method_id(filePromiseProvider:fileNameForType:))]
        fn file_name(
            &self,
            _provider: &NSFilePromiseProvider,
            _file_type: &NSString,
        ) -> Retained<NSString> {
            NSString::from_str(&self.ivars().file.name)
        }
        #[unsafe(method_id(operationQueueForFilePromiseProvider:))]
        fn operation_queue(&self, _provider: &NSFilePromiseProvider) -> Retained<NSOperationQueue> {
            self.ivars().queue.clone()
        }
        #[unsafe(method(filePromiseProvider:writePromiseToURL:completionHandler:))]
        fn write_promise(
            &self,
            _provider: &NSFilePromiseProvider,
            url: &NSURL,
            handler: &DynBlock<dyn Fn(*mut NSError)>,
        ) {
            // 完成回调可能让AppKit释放provider，保持self直到Rust处理完整结束。
            let _keep_alive = unsafe { Retained::retain(self as *const Self as *mut Self) };
            if self.ivars().written.swap(true, Ordering::AcqRel) {
                let error = unsafe {
                    NSError::errorWithDomain_code_userInfo(
                        ns_string!("TermvexaFilePromise"),
                        1,
                        None,
                    )
                };
                handler.call((&*error as *const NSError as *mut NSError,));
                return;
            }
            {
                let mut completion = self.ivars().completion.lock().unwrap();
                // 收尾后系统仍可能持有provider；迟到回调不能重新启动传输。
                if completion.sender.is_none() {
                    let error = unsafe {
                        NSError::errorWithDomain_code_userInfo(
                            ns_string!("TermvexaFilePromise"),
                            3,
                            None,
                        )
                    };
                    drop(completion);
                    handler.call((&*error as *const NSError as *mut NSError,));
                    return;
                }
                completion.active += 1;
                completion.changed_at = std::time::Instant::now();
                completion.requested = true;
                completion.changed.notify_one();
            }
            // URL由Finder提供，不能接受网页伪造的目标路径；保留原始文件系统字节。
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                if !url.isFileURL() {
                    return Err("Finder未提供本地文件目标".into());
                }
                let bytes =
                    unsafe { CStr::from_ptr(url.fileSystemRepresentation().as_ptr()) }.to_bytes();
                let path = PathBuf::from(std::ffi::OsStr::from_bytes(bytes));
                (self.ivars().file.write)(path)
            }))
            .unwrap_or_else(|_| Err("原生文件下载回调异常".into()));
            let error = result.as_ref().err().map(|_| unsafe {
                NSError::errorWithDomain_code_userInfo(ns_string!("TermvexaFilePromise"), 2, None)
            });
            handler.call((error.as_ref().map_or(std::ptr::null_mut(), |e| {
                &**e as *const NSError as *mut NSError
            }),));
            let mut completion = self.ivars().completion.lock().unwrap();
            if let Err(error) = result {
                completion.error.get_or_insert(error);
            }
            completion.active = completion.active.saturating_sub(1);
            completion.remaining = completion.remaining.saturating_sub(1);
            completion.finish();
        }
    }
);
impl FilePromiseDelegate {
    fn new(
        file: PromisedFile,
        completion: Arc<Mutex<Completion>>,
        queue: Retained<NSOperationQueue>,
    ) -> Retained<Self> {
        let this = Self::alloc().set_ivars(PromiseIvars {
            file,
            completion,
            queue,
            written: AtomicBool::new(false),
        });
        // 调用NSObject初始化，不跨线程传递AppKit视图。
        unsafe { msg_send![super(this), init] }
    }
}

struct SourceIvars {
    completion: Arc<Mutex<Completion>>,
}

thread_local! { static DRAG_SOURCES: RefCell<Vec<Retained<DragSource>>> = const { RefCell::new(Vec::new()) }; }

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = SourceIvars]
    struct DragSource;
    unsafe impl NSObjectProtocol for DragSource {}
    unsafe impl NSDraggingSource for DragSource {
        #[unsafe(method(draggingSession:sourceOperationMaskForDraggingContext:))]
        fn operation(
            &self,
            _session: &NSDraggingSession,
            context: NSDraggingContext,
        ) -> NSDragOperation {
            // 本软件内部不是下载落点；仅允许外部应用接收文件承诺。
            if context == NSDraggingContext::WithinApplication {
                NSDragOperation::None
            } else {
                NSDragOperation::Copy
            }
        }
        #[unsafe(method(draggingSession:endedAtPoint:operation:))]
        fn ended(&self, _session: &NSDraggingSession, _point: NSPoint, operation: NSDragOperation) {
            // 从全局保活表移除源时，当前方法仍持有自己的强引用。
            let _keep_alive = unsafe { Retained::retain(self as *const Self as *mut Self) };
            let mut completion = self.ivars().completion.lock().unwrap();
            completion.ended = true;
            if !operation.contains(NSDragOperation::Copy) {
                if let Some(sender) = completion.sender.take() {
                    let _ = sender.send(Ok(false));
                }
            }
            completion.finish();
            drop(completion);
            // 系统拖动会话结束后释放源；文件承诺的delegate由provider单独强持有。
            DRAG_SOURCES.with(|sources| {
                sources
                    .borrow_mut()
                    .retain(|source| !std::ptr::eq(&**source, self))
            });
        }
    }
);
impl DragSource {
    fn new(mtm: MainThreadMarker, completion: Arc<Mutex<Completion>>) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(SourceIvars { completion });
        unsafe { msg_send![super(this), init] }
    }
}

pub async fn start_drag(
    window: tauri::WebviewWindow,
    files: Vec<PromisedFile>,
    outgoing: Arc<super::OutgoingDrag>,
) -> DragResult {
    let (sender, mut receiver) = oneshot::channel();
    let changed = Arc::new(Notify::new());
    let completion = Arc::new(Mutex::new(Completion {
        ended: false,
        remaining: files.len(),
        active: 0,
        requested: false,
        changed: changed.clone(),
        changed_at: std::time::Instant::now(),
        error: None,
        sender: Some(sender),
    }));
    let state = completion.clone();
    let native_window = window.clone();
    window
        .run_on_main_thread(move || {
            // 命令已因内部Drop取消时，不允许排队的主线程回调再启动旧手势。
            if state
                .lock()
                .unwrap()
                .sender
                .as_ref()
                .is_none_or(|sender| sender.is_closed())
            {
                return;
            }
            // IPC排队期间已松开鼠标属于取消手势，不产生失败的传输记录。
            if NSEvent::pressedMouseButtons() & 1 == 0 {
                if let Some(sender) = state.lock().unwrap().sender.take() {
                    let _ = sender.send(Ok(false));
                }
                return;
            }
            let result = begin(&native_window, files, state.clone(), &outgoing);
            if let Err(error) = result {
                if let Some(sender) = state.lock().unwrap().sender.take() {
                    let _ = sender.send(Err(error));
                }
            }
        })
        .map_err(|error| error.to_string())?;
    loop {
        tokio::select! {
            result = &mut receiver => return result.map_err(|_| "原生拖动会话已结束".to_owned())?,
            _ = changed.notified() => {},
            _ = tokio::time::sleep(std::time::Duration::from_millis(50)) => {
                // 刚开始拖动就松开时，AppKit可能没有发出源的ended回调。
                // 独立核对真实鼠标，不能把恢复操作的条件放在ended之后。
                inspect_released_gesture(&window, completion.clone())?;
                {
                    let mut state = completion.lock().unwrap();
                    if state.ended && state.active == 0 && state.remaining > 0
                        && state.changed_at.elapsed() >= std::time::Duration::from_secs(5)
                    {
                        let result = if state.requested {
                            Err("Finder未接收全部拖出条目，已结束等待；已下载的内容保留".into())
                        } else {
                            Ok(false)
                        };
                        if let Some(sender) = state.sender.take() {
                            let _ = sender.send(result);
                        }
                    }
                }
            },
        }
    }
}

fn inspect_released_gesture(
    window: &tauri::WebviewWindow,
    completion: Arc<Mutex<Completion>>,
) -> Result<(), String> {
    {
        let state = completion.lock().unwrap();
        if state.requested || state.sender.is_none() {
            return Ok(());
        }
    }
    let window = window.clone();
    let native_window = window.clone();
    window
        .run_on_main_thread(move || {
            if NSEvent::pressedMouseButtons() & 1 != 0 {
                return;
            }
            let Some(mtm) = MainThreadMarker::new() else {
                return;
            };
            let inside = native_window
                .ns_view()
                .ok()
                .and_then(|view| unsafe { (view as *const NSView).as_ref() })
                .and_then(|view| view.window())
                .is_some_and(|window| {
                    NSWindow::windowNumberAtPoint_belowWindowWithWindowNumber(
                        NSEvent::mouseLocation(),
                        0,
                        mtm,
                    ) == window.windowNumber()
                });
            let mut state = completion.lock().unwrap();
            // 只有尚未被外部请求内容的手势允许按松键取消，不能打断真实下载。
            if state.requested || state.sender.is_none() {
                return;
            }
            let newly_ended = !state.ended;
            state.ended = true;
            if inside {
                if let Some(sender) = state.sender.take() {
                    let _ = sender.send(Ok(false));
                }
            }
            // 外部松键保留兑现宽限期；原生ended回调仍可提供真正的操作结果。
            if newly_ended || inside {
                state.finish();
            }
        })
        .map_err(|error| error.to_string())
}

fn begin(
    window: &tauri::WebviewWindow,
    files: Vec<PromisedFile>,
    completion: Arc<Mutex<Completion>>,
    outgoing: &super::OutgoingDrag,
) -> Result<(), String> {
    let mtm = MainThreadMarker::new().ok_or("原生拖动必须在主线程启动")?;
    // ns_view指针属于当前窗口，仅在主线程和当前调用内借用。
    let view =
        unsafe { (window.ns_view().map_err(|error| error.to_string())? as *const NSView).as_ref() }
            .ok_or("窗口视图不可用")?;
    let native_window = view.window().ok_or("拖动视图不属于活动窗口")?;
    let location = native_window.mouseLocationOutsideOfEventStream();
    let timestamp = NSApplication::sharedApplication(mtm)
        .currentEvent()
        .map(|event| event.timestamp())
        .unwrap_or(0.0);
    // IPC抵达时currentEvent可能已是WebKit事件；基于仍按下的真实鼠标重建原生拖动事件。
    let event = NSEvent::mouseEventWithType_location_modifierFlags_timestamp_windowNumber_context_eventNumber_clickCount_pressure(
        NSEventType::LeftMouseDragged, location, NSEventModifierFlags::empty(),
        timestamp, native_window.windowNumber(), None, 0, 1, 1.0,
    ).ok_or("无法创建原生文件拖动事件")?;
    let queue = NSOperationQueue::new();
    queue.setMaxConcurrentOperationCount(1);
    let point = view.convertPoint_fromView(event.locationInWindow(), None);
    let mut items = Vec::with_capacity(files.len());
    for file in files {
        let file_type = if file.directory {
            ns_string!("public.folder")
        } else {
            ns_string!("public.data")
        };
        let delegate = FilePromiseDelegate::new(file, completion.clone(), queue.clone());
        let provider = NSFilePromiseProvider::initWithFileType_delegate(
            NSFilePromiseProvider::alloc(),
            file_type,
            ProtocolObject::from_ref(&*delegate),
        );
        // delegate属性是weak；userInfo强引用确保承诺落盘完成前delegate仍然有效。
        unsafe {
            provider.setUserInfo(Some(&delegate));
        }
        let item = NSDraggingItem::initWithPasteboardWriter(
            NSDraggingItem::alloc(),
            ProtocolObject::from_ref(&*provider),
        );
        let image = NSImage::imageNamed(ns_string!("NSMultipleDocuments"));
        unsafe {
            item.setDraggingFrame_contents(
                NSRect::new(point, NSSize::new(32.0, 32.0)),
                image.as_deref().map(|image| &***image),
            );
        }
        items.push(item);
    }
    outgoing.mark_native_started();
    let source = DragSource::new(mtm, completion);
    // AppKit不持有拖动源；即使命令已取消，仍保留到ended，避免迟到回调访问已释放对象。
    DRAG_SOURCES.with(|sources| sources.borrow_mut().push(source.clone()));
    view.beginDraggingSessionWithItems_event_source(
        &NSArray::from_retained_slice(&items),
        &event,
        ProtocolObject::from_ref(&*source),
    );
    Ok(())
}

/// WebKit收到了原始mouseDown，原生拖动则接管mouseUp；显式补齐松键，避免下次点击被当作旧手势。
pub async fn finish_drag(window: &tauri::WebviewWindow) -> Result<(), String> {
    let (sender, receiver) = oneshot::channel();
    window.with_webview(move |webview| {
        let result: Result<(), String> = (|| {
            // 仍在按住时不能伪造松键，也不抢夺外部文件管理器焦点。
            if NSEvent::pressedMouseButtons() & 1 != 0 { return Ok(()); }
            let view = unsafe {
                (webview.inner() as *const NSView).as_ref()
            }.ok_or("窗口视图不可用")?;
            let window = view.window().ok_or("窗口已关闭")?;
            let mtm = MainThreadMarker::new().ok_or("鼠标收尾必须在主线程")?;
            let timestamp = NSApplication::sharedApplication(mtm).currentEvent()
                .map(|event| event.timestamp()).unwrap_or(0.0);
            let event = NSEvent::mouseEventWithType_location_modifierFlags_timestamp_windowNumber_context_eventNumber_clickCount_pressure(
                NSEventType::LeftMouseUp, window.mouseLocationOutsideOfEventStream(),
                NSEventModifierFlags::empty(), timestamp, window.windowNumber(), None, 0, 1, 0.0,
            ).ok_or("无法创建鼠标松键事件")?;
            // 向网页当前响应者补齐松键，不改变窗口焦点。
            if let Some(responder) = window.firstResponder() {
                responder.mouseUp(&event);
            } else {
                view.mouseUp(&event);
            }
            Ok(())
        })();
        let _ = sender.send(result);
    }).map_err(|error| error.to_string())?;
    receiver.await.map_err(|_| "鼠标收尾已中断".to_owned())?
}
