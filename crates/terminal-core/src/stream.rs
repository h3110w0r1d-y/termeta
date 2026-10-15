use crate::*;
use std::{
    collections::VecDeque,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};
use tokio::sync::{Notify, OwnedSemaphorePermit, Semaphore, mpsc};

pub const MAX_OUTPUT_PAYLOAD: usize = 32768;
const OUTPUT_BUDGET: usize = 512 * 1024;
const OUTPUT_CREDIT: u64 = 256 * 1024;
struct OutputChunk {
    bytes: Bytes,
    _permit: OwnedSemaphorePermit,
}
#[derive(Clone)]
pub struct OutputSender {
    sender: mpsc::Sender<OutputChunk>,
    budget: Arc<Semaphore>,
    cancel: CancellationToken,
}
impl OutputSender {
    pub async fn send(&self, bytes: Bytes) -> Result<()> {
        if bytes.is_empty() || bytes.len() > MAX_OUTPUT_PAYLOAD {
            return Err(SessionError::new(
                "PROTOCOL_MISMATCH",
                "output",
                "输出块大小无效",
            ));
        }
        // 每块都受字节与消息数双重限额，SSH 驱动等待此 future 时仍须选择控制分支。
        for chunk in bytes.chunks(MAX_OUTPUT_PAYLOAD) {
            let permit = tokio::select! { biased; _ = self.cancel.cancelled() => return Err(SessionError::cancelled()), permit = self.budget.clone().acquire_many_owned(chunk.len() as u32) => permit.map_err(|_| SessionError::cancelled())? };
            let message = OutputChunk {
                bytes: Bytes::copy_from_slice(chunk),
                _permit: permit,
            };
            tokio::select! { biased; _ = self.cancel.cancelled() => return Err(SessionError::cancelled()), result = self.sender.send(message) => result.map_err(|_| SessionError::cancelled())? }
        }
        Ok(())
    }
}
#[derive(Default)]
struct Credit {
    sent: u64,
    consumed: u64,
    endpoints: VecDeque<u64>,
    retired: bool,
}
pub struct OutputFlow {
    credit: Mutex<Credit>,
    progress: Notify,
    failure: Mutex<Option<SessionError>>,
    submitted: AtomicU64,
}
impl OutputFlow {
    pub(crate) fn submitted_offset(&self) -> u64 {
        self.submitted.load(Ordering::Acquire)
    }
    pub(crate) fn take_error(&self) -> Option<SessionError> {
        self.failure.lock().unwrap().take()
    }
    pub(crate) fn retire(&self) {
        let mut credit = self.credit.lock().unwrap();
        credit.endpoints.clear();
        credit.endpoints.shrink_to_fit();
        credit.retired = true;
    }
    fn fail(&self, code: &str, message: &str) {
        *self.failure.lock().unwrap() = Some(SessionError::new(code, "stream", message));
    }
    pub fn ack(&self, offset: u64) -> Result<()> {
        let mut credit = self.credit.lock().unwrap();
        if credit.retired {
            return Err(SessionError::new(
                "NOT_FOUND",
                "stream",
                "输出确认窗口已过期",
            ));
        }
        if offset == credit.consumed {
            return Ok(());
        }
        if offset < credit.consumed || offset > credit.sent || !credit.endpoints.contains(&offset) {
            return Err(SessionError::new(
                "PROTOCOL_MISMATCH",
                "stream",
                "输出确认偏移无效",
            ));
        }
        credit.consumed = offset;
        while credit.endpoints.front().is_some_and(|end| *end <= offset) {
            credit.endpoints.pop_front();
        }
        self.progress.notify_one();
        Ok(())
    }
}
pub(crate) fn output_stream(
    sink: Arc<dyn SessionSink>,
    cancel: CancellationToken,
) -> (
    OutputSender,
    Arc<OutputFlow>,
    tokio::task::JoinHandle<(u64, bool)>,
) {
    let (sender, mut receiver) = mpsc::channel::<OutputChunk>(32);
    let output = OutputSender {
        sender,
        budget: Arc::new(Semaphore::new(OUTPUT_BUDGET)),
        cancel: cancel.clone(),
    };
    let flow = Arc::new(OutputFlow {
        credit: Mutex::new(Credit::default()),
        progress: Notify::new(),
        failure: Mutex::new(None),
        submitted: AtomicU64::new(0),
    });
    let worker_flow = flow.clone();
    let worker = tokio::spawn(async move {
        let mut sequence = 0u32;
        let mut offset = 0u64;
        loop {
            let message = tokio::select! { biased; _ = cancel.cancelled() => return (offset, false), message = receiver.recv() => match message { Some(message) => message, None => return (offset, true) } };
            loop {
                let notified = worker_flow.progress.notified();
                let available = {
                    let c = worker_flow.credit.lock().unwrap();
                    c.sent - c.consumed + message.bytes.len() as u64 <= OUTPUT_CREDIT
                };
                if available {
                    break;
                }
                tokio::select! { biased; _ = cancel.cancelled() => return (offset, false), result = tokio::time::timeout(Duration::from_secs(300), notified) => if result.is_err() { worker_flow.fail("CONSUMER_STALLED", "终端输出超过五分钟未被消费，已关闭连接"); cancel.cancel(); return (offset, false); } }
            }
            let Some(next_sequence) = sequence.checked_add(1) else {
                worker_flow.fail("PROTOCOL_MISMATCH", "输出帧序号已耗尽，请重新连接");
                cancel.cancel();
                return (offset, false);
            };
            let Some(next_offset) = offset.checked_add(message.bytes.len() as u64) else {
                cancel.cancel();
                return (offset, false);
            };
            let mut frame = vec![0; 24 + message.bytes.len()];
            frame[0] = 1;
            frame[4..8].copy_from_slice(&next_sequence.to_le_bytes());
            frame[8..16].copy_from_slice(&next_offset.to_le_bytes());
            frame[16..20].copy_from_slice(&(message.bytes.len() as u32).to_le_bytes());
            frame[24..].copy_from_slice(&message.bytes);
            // 信用在 Channel 提交前登记，允许同步消费者立即确认，发送失败则会话立即结束。
            {
                let mut c = worker_flow.credit.lock().unwrap();
                c.sent = next_offset;
                c.endpoints.push_back(next_offset);
            }
            if sink.output(frame).is_err() {
                worker_flow.fail("CONSUMER_STALLED", "终端输出通道已失效");
                cancel.cancel();
                return (offset, false);
            }
            worker_flow.submitted.store(next_offset, Ordering::Release);
            offset = next_offset;
            sequence = next_sequence;
        }
    });
    (output, flow, worker)
}
