//! 显式指定设备的接收诊断入口，复用真实会话、转码、输出帧与ACK链路，不发送应用数据。
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use terminal_core::*;
use terminal_serial::SerialFactory;

#[derive(Default)]
struct Capture {
    frames: Mutex<Vec<Vec<u8>>>,
}

struct CloseOnDrop {
    manager: SessionManager,
    session_id: String,
}
impl Drop for CloseOnDrop {
    fn drop(&mut self) {
        // 解析或ACK失败时也必须取消会话，避免阻塞设备线程延迟进程退出。
        let _ = self.manager.close("serial-diagnostic", &self.session_id);
    }
}
impl SessionSink for Capture {
    fn output(&self, frame: Vec<u8>) -> Result<()> {
        self.frames.lock().unwrap().push(frame);
        Ok(())
    }
    fn event(&self, event: SessionEvent) -> Result<()> {
        match event.payload {
            EventPayload::StateChanged { snapshot } => println!("状态：{:?}", snapshot.state),
            EventPayload::Ended { error, .. } => println!("会话结束：{error:?}"),
            _ => {}
        }
        Ok(())
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("用法：serial_monitor <设备路径> <波特率> [观察秒数]")?;
    let baud_rate: u32 = args.next().ok_or("请显式指定波特率")?.parse()?;
    let seconds: u64 = args.next().as_deref().unwrap_or("6").parse()?;
    if !(1..=30).contains(&seconds) {
        return Err("观察秒数须为1至30".into());
    }
    let capture = Arc::new(Capture::default());
    let manager = SessionManager::new(Arc::new(SerialFactory));
    let session = manager.open(
        "serial-diagnostic",
        "49f653ee-5682-425a-b003-9f4601b7e0f6".into(),
        "10a34e6e-73f6-454a-bee5-a61885ac34f4".into(),
        SessionConfig::Serial(SerialConfig {
            path,
            baud_rate,
            charset: "UTF-8".into(),
            data_bits: 8,
            stop_bits: 1,
            flow_control: SerialFlowControl::None,
            parity: SerialParity::None,
        }),
        TerminalSize { cols: 80, rows: 24 },
        capture.clone(),
    )?;
    let _cleanup = CloseOnDrop {
        manager: manager.clone(),
        session_id: session.session_id.clone(),
    };
    let deadline = Instant::now() + Duration::from_secs(seconds);
    let mut preview = Vec::new();
    let mut total = 0;
    while Instant::now() < deadline {
        let frames = std::mem::take(&mut *capture.frames.lock().unwrap());
        for frame in frames {
            if frame.len() < 24 {
                return Err("输出帧头不完整".into());
            }
            let offset = u64::from_le_bytes(frame[8..16].try_into()?);
            // 用转义文本输出预览，避免设备控制序列改变诊断终端状态。
            let payload = &frame[24..];
            if preview.len() < 16384 {
                let length = payload.len().min(16384 - preview.len());
                println!(
                    "收到{}字节：{:?}",
                    payload.len(),
                    String::from_utf8_lossy(&payload[..length])
                );
                preview.extend_from_slice(&payload[..length]);
            }
            total += payload.len();
            manager.ack("serial-diagnostic", &session.session_id, offset)?;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    manager.close("serial-diagnostic", &session.session_id)?;
    let deadline = Instant::now() + Duration::from_secs(4);
    while !manager
        .get("serial-diagnostic", &session.session_id)?
        .state
        .terminal()
    {
        if Instant::now() >= deadline {
            return Err("会话未按时关闭".into());
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    println!(
        "总计{total}字节，预览包含ESP-ROM：{}",
        preview.windows(7).any(|part| part == b"ESP-ROM")
    );
    if total == 0 {
        return Err("观察期未收到串口输出".into());
    }
    Ok(())
}
