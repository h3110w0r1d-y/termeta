//! SFTP 操作与原生回调之间的取消、清单及交付状态，不保存文件正文。
use super::manifest::{self, Manifest, Root};
use crate::{file_drag::OutgoingDrag, files::connected, state::AppState};
use std::{
    collections::HashMap,
    ops::Range,
    sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
use tauri::{AppHandle, Manager, ipc::Channel};
use terminal_core::{CancellationToken, Result, SessionError};
use terminal_files::{FileReader, TransferProgress, file_error};
use terminal_sftp::{RemoteFile, SftpBrowser};
use windows::core::HRESULT;

pub(super) type Remote = Arc<RemoteReader>;

pub(super) struct RemoteReader {
    pub file: tokio::sync::Mutex<Option<RemoteFile>>,
    runtime: tokio::runtime::Handle,
}
impl Drop for RemoteReader {
    fn drop(&mut self) {
        if let Some(mut file) = self.file.get_mut().take() {
            // COM 析构线程不等待网络，也不直接触发依赖 Tokio 的 RemoteFile 析构。
            self.runtime.spawn(async move {
                let _ = file.close().await;
            });
        }
    }
}

pub(super) struct Operation {
    pub drag: Option<(HRESULT, u32)>,
    pub async_enabled: bool,
    pub async_started: bool,
    pub end: Option<(HRESULT, u32)>,
    pub performed: Option<u32>,
    pub sealed: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Decision {
    Pending,
    Cancelled,
    Copied,
    Failed(&'static str),
}
impl Operation {
    fn new() -> Self {
        Self {
            drag: None,
            async_enabled: true,
            async_started: false,
            end: None,
            performed: None,
            sealed: false,
        }
    }
    pub fn start(&mut self) -> windows::core::Result<()> {
        if self.sealed {
            return Err(windows::Win32::Foundation::E_ABORT.into());
        }
        if !self.async_enabled || self.async_started {
            return Err(windows::Win32::Foundation::E_FAIL.into());
        }
        self.async_started = true;
        Ok(())
    }
    pub fn end(&mut self, result: HRESULT, effect: u32) -> windows::core::Result<()> {
        if self.sealed {
            return Err(windows::Win32::Foundation::E_ABORT.into());
        }
        if !self.async_started || self.end.is_some() {
            return Err(windows::Win32::Foundation::E_FAIL.into());
        }
        self.end = Some((result, effect));
        Ok(())
    }
    pub fn decision(&self) -> Decision {
        use windows::Win32::{Foundation::DRAGDROP_S_DROP, System::Ole::DROPEFFECT_COPY};
        if self.sealed {
            return Decision::Cancelled;
        }
        let Some((result, effect)) = self.drag else {
            return Decision::Pending;
        };
        if result.is_err() {
            return Decision::Failed("Windows 原生拖出失败，目标可能保留部分文件");
        }
        if result != DRAGDROP_S_DROP || effect != DROPEFFECT_COPY.0 {
            return Decision::Cancelled;
        }
        if self
            .performed
            .is_some_and(|effect| effect != DROPEFFECT_COPY.0)
        {
            return Decision::Failed("接收端未确认复制成功，远端源文件已保留");
        }
        if self.async_started {
            let Some((result, effect)) = self.end else {
                return Decision::Pending;
            };
            if result.is_err() || effect != DROPEFFECT_COPY.0 {
                return Decision::Failed("接收端未完成拖出下载，目标可能保留部分文件");
            }
        }
        Decision::Copied
    }
}

#[derive(Default)]
struct Coverage(Vec<Range<u64>>);
impl Coverage {
    fn add(&mut self, mut range: Range<u64>) {
        if range.is_empty() {
            return;
        }
        let first = self
            .0
            .partition_point(|existing| existing.end < range.start);
        let mut end = first;
        while let Some(existing) = self.0.get(end) {
            if existing.start > range.end {
                break;
            }
            range.start = range.start.min(existing.start);
            range.end = range.end.max(existing.end);
            end += 1;
        }
        self.0.splice(first..end, [range]);
    }
    fn complete(&self, size: u64) -> bool {
        size == 0
            || self
                .0
                .first()
                .is_some_and(|range| range.start == 0 && range.end == size)
    }
}

struct Progress {
    coverage: HashMap<usize, Coverage>,
    intervals: usize,
    last: Instant,
}

pub(super) struct Transfer {
    pub runtime: tokio::runtime::Handle,
    pub cancel: CancellationToken,
    pub outgoing: Arc<OutgoingDrag>,
    pub operation: Mutex<Operation>,
    pub error: Mutex<Option<String>>,
    pub streams: AtomicUsize,
    pub waiting: AtomicUsize,
    pub descriptors_served: AtomicBool,
    pub manifest: tokio::sync::OnceCell<Arc<Manifest>>,
    app: AppHandle,
    owner: String,
    session_id: String,
    browser: Arc<SftpBrowser>,
    roots: Vec<Root>,
    channel: Channel<TransferProgress>,
    accepted: AtomicBool,
    progress: Mutex<Progress>,
    // 仅本次传输串行化远端请求；不持有全局状态锁等待网络或 Channel。
    gate: tokio::sync::Mutex<()>,
    readers: Mutex<Vec<Weak<RemoteReader>>>,
}

impl Transfer {
    pub fn new(
        app: AppHandle,
        owner: String,
        session_id: String,
        browser: Arc<SftpBrowser>,
        roots: Vec<Root>,
        cancel: CancellationToken,
        outgoing: Arc<OutgoingDrag>,
        channel: Channel<TransferProgress>,
    ) -> Arc<Self> {
        Arc::new(Self {
            runtime: tokio::runtime::Handle::current(),
            cancel,
            outgoing,
            operation: Mutex::new(Operation::new()),
            error: Mutex::new(None),
            streams: AtomicUsize::new(0),
            waiting: AtomicUsize::new(0),
            descriptors_served: AtomicBool::new(false),
            manifest: tokio::sync::OnceCell::new(),
            app,
            owner,
            session_id,
            browser,
            roots,
            channel,
            accepted: AtomicBool::new(false),
            progress: Mutex::new(Progress {
                coverage: HashMap::new(),
                intervals: 0,
                last: Instant::now(),
            }),
            gate: tokio::sync::Mutex::new(()),
            readers: Mutex::new(Vec::new()),
        })
    }

    pub fn cancelled(&self) -> bool {
        self.cancel.is_cancelled() || self.outgoing.is_cancelled()
    }

    fn check(&self) -> Result<()> {
        if self.cancelled() {
            return Err(SessionError::cancelled());
        }
        connected(&self.app.state::<AppState>(), &self.owner, &self.session_id)
    }

    pub fn fail(&self, message: String) {
        if !self.cancelled() {
            self.error.lock().unwrap().get_or_insert(message);
        }
    }

    pub async fn get_manifest(&self) -> Result<Arc<Manifest>> {
        self.check()?;
        self.manifest
            .get_or_try_init(|| async {
                let _gate = self.gate.lock().await;
                self.check()?;
                manifest::scan(&self.browser, &self.roots, &self.cancel)
                    .await
                    .map(Arc::new)
            })
            .await
            .cloned()
    }

    pub async fn open(&self, index: usize) -> Result<Remote> {
        let _gate = self.gate.lock().await;
        self.check()?;
        let manifest = self
            .manifest
            .get()
            .ok_or_else(|| file_error("拖出目录清单尚未建立"))?;
        let entry = manifest
            .entries
            .get(index)
            .ok_or_else(|| file_error("拖出文件索引无效"))?;
        if entry.directory {
            return Err(file_error("目录不提供文件内容流"));
        }
        {
            let mut readers = self.readers.lock().unwrap();
            readers.retain(|reader| reader.strong_count() > 0);
            if readers.len() >= 128 {
                return Err(file_error("同时打开的拖出文件过多"));
            }
        }
        // 不丢弃正在 OPEN 的 future；收到句柄后即使已取消也明确关闭。
        let mut file = self.browser.reader(&entry.remote).await?;
        let size = file.size().await;
        if self.cancelled() || !size.as_ref().is_ok_and(|size| *size == Some(entry.size)) {
            let _ = file.close().await;
            self.check()?;
            return Err(file_error("拖出源大小已变化或无法核对，请重新拖动"));
        }
        let remote = Arc::new(RemoteReader {
            file: tokio::sync::Mutex::new(Some(file)),
            runtime: self.runtime.clone(),
        });
        self.readers.lock().unwrap().push(Arc::downgrade(&remote));
        Ok(remote)
    }

    pub async fn read(
        &self,
        remote: &Remote,
        position: u64,
        size: u64,
        buffer: &mut [u8],
    ) -> Result<usize> {
        let _gate = self.gate.lock().await;
        self.check()?;
        let mut file = remote.file.lock().await;
        let file = file.as_mut().ok_or_else(SessionError::cancelled)?;
        file.seek_to(position);
        let count = tokio::select! {
            biased;
            _ = self.cancel.cancelled() => return Err(SessionError::cancelled()),
            _ = self.outgoing.cancelled() => return Err(SessionError::cancelled()),
            result = file.read_chunk(buffer) => result?,
        };
        if position + count as u64 == size && file.size().await? != Some(size) {
            return Err(file_error("拖出源在读取期间改变大小，目标可能保留部分文件"));
        }
        self.check()?;
        Ok(count)
    }

    pub fn delivered(&self, index: usize, range: Range<u64>) -> Result<()> {
        {
            let mut progress = self.progress.lock().unwrap();
            // 非顺序读取也按区间去重；拒绝恶意碎片化请求，避免跟踪元数据无界增长。
            if progress.intervals >= 65536 {
                return Err(file_error("原生文件读取过于碎片化，传输已停止"));
            }
            let coverage = progress.coverage.entry(index).or_default();
            let before = coverage.0.len();
            coverage.add(range);
            let after = coverage.0.len();
            progress.intervals = progress.intervals + after - before;
        }
        self.report(Some(index), false, false);
        Ok(())
    }

    pub fn accept(&self) {
        self.accepted.store(true, Ordering::Release);
        self.report(None, true, false);
    }

    pub fn report(&self, index: Option<usize>, force: bool, finished: bool) {
        if !self.accepted.load(Ordering::Acquire) {
            return;
        }
        let value = {
            let mut progress = self.progress.lock().unwrap();
            if !force && progress.last.elapsed() < Duration::from_millis(150) {
                return;
            }
            progress.last = Instant::now();
            let manifest = self.manifest.get();
            let entry =
                index.and_then(|index| manifest.and_then(|manifest| manifest.entries.get(index)));
            let completed = manifest.map(|manifest| {
                manifest
                    .entries
                    .iter()
                    .enumerate()
                    .filter(|(index, entry)| {
                        if entry.directory {
                            finished
                        } else {
                            entry.size == 0
                                || progress
                                    .coverage
                                    .get(index)
                                    .is_some_and(|coverage| coverage.complete(entry.size))
                        }
                    })
                    .count()
            });
            let transferred = index
                .and_then(|index| progress.coverage.get(&index))
                .map(|coverage| {
                    coverage
                        .0
                        .iter()
                        .map(|range| range.end - range.start)
                        .sum::<u64>()
                })
                .unwrap_or(0);
            TransferProgress {
                transferred: transferred.to_string(),
                total: entry.map(|entry| entry.size.to_string()),
                current_name: entry
                    .map(|entry| entry.relative.clone())
                    .or_else(|| self.roots.first().map(|root| root.name.clone())),
                completed_files: completed,
                total_files: manifest.map(|manifest| manifest.entries.len()),
            }
        };
        if self.channel.send(value).is_err() {
            self.cancel.cancel();
        }
    }

    pub fn all_delivered(&self) -> bool {
        let Some(manifest) = self.manifest.get() else {
            return false;
        };
        if !self.descriptors_served.load(Ordering::Acquire) {
            return false;
        }
        let progress = self.progress.lock().unwrap();
        manifest.entries.iter().enumerate().all(|(index, entry)| {
            entry.directory
                || entry.size == 0
                || progress
                    .coverage
                    .get(&index)
                    .is_some_and(|coverage| coverage.complete(entry.size))
        })
    }

    pub async fn close(&self) -> Result<()> {
        self.cancel.cancel();
        self.outgoing.finish();
        self.operation.lock().unwrap().sealed = true;
        // 与清单/OPEN/READ 共用门闩，迟到 COM 请求只能看到取消，不能复活会话。
        let _gate = self.gate.lock().await;
        let readers: Vec<_> = self
            .readers
            .lock()
            .unwrap()
            .drain(..)
            .filter_map(|reader| reader.upgrade())
            .collect();
        let mut failed = false;
        for reader in readers {
            if let Some(mut file) = reader.file.lock().await.take() {
                failed |= file.close().await.is_err();
            }
        }
        if failed {
            return Err(file_error("拖出读取已结束，但部分远端句柄关闭失败"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Coverage, Decision, Operation};
    use windows::Win32::{
        Foundation::{DRAGDROP_S_DROP, E_ABORT, E_FAIL, S_OK},
        System::Ole::{DROPEFFECT_COPY, DROPEFFECT_MOVE},
    };

    #[test]
    fn drop_is_not_async_completion_and_failure_cannot_become_success() {
        let mut operation = Operation::new();
        operation.start().unwrap();
        operation.drag = Some((DRAGDROP_S_DROP, DROPEFFECT_COPY.0));
        assert_eq!(operation.decision(), Decision::Pending);
        operation.end(E_FAIL, DROPEFFECT_COPY.0).unwrap();
        assert!(matches!(operation.decision(), Decision::Failed(_)));
        assert!(operation.end(S_OK, DROPEFFECT_COPY.0).is_err());
    }

    #[test]
    fn early_end_waits_for_gesture_and_sealing_rejects_late_callbacks() {
        let mut operation = Operation::new();
        operation.start().unwrap();
        operation.end(S_OK, DROPEFFECT_COPY.0).unwrap();
        assert_eq!(operation.decision(), Decision::Pending);
        operation.drag = Some((DRAGDROP_S_DROP, DROPEFFECT_COPY.0));
        assert_eq!(operation.decision(), Decision::Copied);
        operation.sealed = true;
        assert_eq!(operation.decision(), Decision::Cancelled);
        assert_eq!(operation.start().unwrap_err().code(), E_ABORT);
        assert_eq!(
            operation.end(S_OK, DROPEFFECT_COPY.0).unwrap_err().code(),
            E_ABORT
        );
    }

    #[test]
    fn move_is_never_a_successful_copy() {
        let mut operation = Operation::new();
        operation.drag = Some((DRAGDROP_S_DROP, DROPEFFECT_MOVE.0));
        assert_eq!(operation.decision(), Decision::Cancelled);
    }

    #[test]
    fn repeated_and_out_of_order_reads_count_only_delivered_bytes() {
        let mut coverage = Coverage::default();
        coverage.add(5..10);
        assert!(!coverage.complete(10));
        coverage.add(7..9);
        coverage.add(0..3);
        coverage.add(3..5);
        assert_eq!(coverage.0, vec![0..10]);
        assert!(coverage.complete(10));
        coverage.add(0..10);
        assert_eq!(coverage.0, vec![0..10]);
    }

    #[test]
    fn empty_file_is_complete_but_seeking_to_end_is_not_delivery() {
        let mut coverage = Coverage::default();
        coverage.add(10..10);
        assert!(coverage.complete(0));
        assert!(!coverage.complete(10));
    }
}
