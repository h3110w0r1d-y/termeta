//! 串口驱动：独占设备、限时阻塞读写与有界桥接，不在异步执行器直接操作设备。
mod codec;
mod device_path;
#[cfg(target_os = "macos")]
mod macos;

use async_trait::async_trait;
use codec::Codec;
use serde::Serialize;
use serialport::{DataBits, FlowControl, Parity, SerialPort, SerialPortType, StopBits};
use std::{
    collections::HashSet,
    io::{ErrorKind, Read, Write},
    sync::{Arc, Mutex, OnceLock},
    time::{Duration, Instant},
};
use terminal_core::*;

const IO_TIMEOUT: Duration = Duration::from_millis(100);
static OPEN_PORTS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SerialPortInfo {
    pub path: String,
    pub description: Option<String>,
}

pub fn list_ports() -> Result<Vec<SerialPortInfo>> {
    let ports = serialport::available_ports().map_err(|_| {
        SessionError::new(
            "SERIAL_ENUM_FAILED",
            "serial",
            "无法枚举串口，请检查系统设备与权限",
        )
    })?;
    #[cfg(target_os = "macos")]
    let ports = macos::dialin_ports(ports)?;
    let mut ports: Vec<_> = ports
        .into_iter()
        .map(|port| SerialPortInfo {
            path: port.port_name,
            description: match port.port_type {
                SerialPortType::UsbPort(info) => info.product.or(info.manufacturer),
                SerialPortType::BluetoothPort => Some("Bluetooth".into()),
                _ => None,
            },
        })
        .collect();
    ports.sort_by(|a, b| a.path.cmp(&b.path));
    ports.dedup_by(|a, b| a.path == b.path);
    Ok(ports)
}

fn io_error(phase: &str) -> SessionError {
    SessionError::new(
        "SERIAL_IO_FAILED",
        phase,
        "串口读写失败，设备可能已拔出或驱动报告错误；未确认输入不会自动重发",
    )
}
fn open_error(error: serialport::Error) -> SessionError {
    let (code, message) = match error.kind() {
        #[cfg(unix)]
        serialport::ErrorKind::NoDevice => (
            "SERIAL_BUSY",
            "串口被占用或无法取得独占访问，请关闭其他串口连接后重试",
        ),
        #[cfg(windows)]
        serialport::ErrorKind::NoDevice => (
            "SERIAL_UNAVAILABLE",
            "串口不可用或已被占用，请刷新设备列表并检查权限",
        ),
        serialport::ErrorKind::Io(ErrorKind::PermissionDenied) => (
            "SERIAL_PERMISSION_DENIED",
            "无权打开串口，请检查系统串口权限或端口占用",
        ),
        serialport::ErrorKind::Io(ErrorKind::NotFound) => (
            "SERIAL_UNAVAILABLE",
            "串口设备不存在或已移除，请核对设备路径并刷新",
        ),
        serialport::ErrorKind::InvalidInput => {
            ("INVALID_CONFIG", "串口驱动不支持所选参数，请调整配置")
        }
        _ => (
            "SERIAL_OPEN_FAILED",
            "无法打开串口，请检查端口占用、设备与串口参数",
        ),
    };
    // 保留受限长度的驱动原因，避免把占用、权限与参数错误都合并为同一提示。
    let detail: String = error
        .description
        .chars()
        .filter(|character| !character.is_control())
        .take(240)
        .collect();
    SessionError::new(code, "opening", &format!("{message}（{detail}）"))
}

struct PortLease(String);
impl PortLease {
    fn acquire(path: &str) -> Result<Self> {
        #[cfg(unix)]
        let key = path.to_owned();
        #[cfg(windows)]
        let key = path.to_ascii_uppercase();
        let mut ports = OPEN_PORTS.get_or_init(Default::default).lock().unwrap();
        if !ports.insert(key.clone()) {
            return Err(SessionError::new(
                "SERIAL_BUSY",
                "opening",
                "此串口已在应用中打开，请先断开原连接",
            ));
        }
        Ok(Self(key))
    }
}
impl Drop for PortLease {
    fn drop(&mut self) {
        OPEN_PORTS.get().unwrap().lock().unwrap().remove(&self.0);
    }
}

struct CancelOnDrop(CancellationToken);
impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        // 管理器可直接丢弃驱动future，两个设备线程仍须在超时后释放句柄。
        self.0.cancel();
    }
}

struct DeviceHandle {
    // 字段按声明顺序析构；先关闭系统句柄，再释放应用内独占登记。
    port: Box<dyn SerialPort>,
    _lease: Arc<PortLease>,
}
impl DeviceHandle {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        self.port.read(bytes)
    }
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.port.write(bytes)
    }
}
type Device = (DeviceHandle, DeviceHandle);
fn open_device(config: &SerialConfig, cancel: &CancellationToken) -> Result<Device> {
    if cancel.is_cancelled() {
        return Err(SessionError::cancelled());
    }
    // 枚举只提供候选项；手输路径须经过平台设备校验，不能打开普通文件。
    let path = device_path::resolve(&config.path)?;
    let lease = Arc::new(PortLease::acquire(&path)?);
    let builder = serialport::new(&path, config.baud_rate)
        .data_bits(match config.data_bits {
            5 => DataBits::Five,
            6 => DataBits::Six,
            7 => DataBits::Seven,
            _ => DataBits::Eight,
        })
        .stop_bits(if config.stop_bits == 2 {
            StopBits::Two
        } else {
            StopBits::One
        })
        .flow_control(match config.flow_control {
            SerialFlowControl::None => FlowControl::None,
            SerialFlowControl::Hardware => FlowControl::Hardware,
            SerialFlowControl::Software => FlowControl::Software,
        })
        .parity(match config.parity {
            SerialParity::None => Parity::None,
            SerialParity::Odd => Parity::Odd,
            SerialParity::Even => Parity::Even,
        })
        // 明确初始DTR状态，避免沿用上一次连接留下的USB串口控制状态。
        .dtr_on_open(false)
        .timeout(IO_TIMEOUT);
    #[cfg(unix)]
    // 固定版本的open_native默认取得TIOCEXCL/flock独占，无需重复设置。
    let mut port: Box<dyn SerialPort> = Box::new(builder.open_native().map_err(open_error)?);
    // Windows的CreateFile由库以零共享标志打开，系统拒绝其他连接。
    #[cfg(windows)]
    let mut port: Box<dyn SerialPort> = Box::new(builder.open_native().map_err(open_error)?);
    if !matches!(config.flow_control, SerialFlowControl::Hardware) {
        // 非硬件流控时解除RTS，避免USB串口控制器让设备停留在复位状态。
        // 与库的DTR初始化一致：没有控制引脚的虚拟/串口设备仍允许连接。
        let _ = port.write_request_to_send(false);
    }
    let writer = port.try_clone().map_err(open_error)?;
    if cancel.is_cancelled() {
        return Err(SessionError::cancelled());
    }
    Ok((
        DeviceHandle {
            port,
            _lease: lease.clone(),
        },
        DeviceHandle {
            port: writer,
            _lease: lease,
        },
    ))
}

pub struct SerialFactory;
#[async_trait]
impl SessionFactory for SerialFactory {
    fn kind(&self) -> &'static str {
        "serial"
    }
    fn capabilities(&self) -> SessionCapabilities {
        SessionCapabilities {
            resize: false,
            pty: false,
            binary_output: true,
            serial_control: false,
        }
    }
    fn validate(&self, config: &SessionConfig) -> Result<()> {
        match config {
            SessionConfig::Serial(config) => config.validate(),
            _ => Err(SessionError::new(
                "UNSUPPORTED_KIND",
                "opening",
                "串口工厂仅接受串口配置",
            )),
        }
    }
    async fn run(&self, config: SessionConfig, mut context: DriverContext) -> Result<SessionEnd> {
        self.validate(&config)?;
        let SessionConfig::Serial(config) = config else {
            unreachable!()
        };
        context.events.state(SessionState::Opening)?;
        let worker_cancel = CancellationToken::new();
        let _guard = CancelOnDrop(worker_cancel.clone());
        let opening_cancel = worker_cancel.clone();
        let charset = config.charset.clone();
        let (mut reader, mut writer) =
            tokio::task::spawn_blocking(move || open_device(&config, &opening_cancel))
                .await
                .map_err(|_| io_error("opening"))??;
        let (output_send, mut output_receive) = tokio::sync::mpsc::channel::<Vec<u8>>(4);
        let (failure_send, mut failure_receive) = tokio::sync::mpsc::channel::<SessionError>(2);
        let read_cancel = worker_cancel.clone();
        let read_failure = failure_send.clone();
        tokio::task::spawn_blocking(move || {
            let mut buffer = [0; 8192];
            while !read_cancel.is_cancelled() {
                match reader.read(&mut buffer) {
                    Ok(0) => {
                        let _ = read_failure.try_send(io_error("output"));
                        break;
                    }
                    Ok(count) => {
                        if output_send.blocking_send(buffer[..count].to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(error)
                        if matches!(
                            error.kind(),
                            ErrorKind::TimedOut | ErrorKind::WouldBlock | ErrorKind::Interrupted
                        ) =>
                    {
                        continue;
                    }
                    Err(_) => {
                        let _ = read_failure.try_send(io_error("output"));
                        break;
                    }
                }
            }
            // 输出通道结束前先关闭读取句柄，重开不会追上尚未析构的设备。
            drop(reader);
        });
        let (input_send, mut input_receive) = tokio::sync::mpsc::channel::<InputChunk>(1);
        let write_cancel = worker_cancel.clone();
        let input_charset = charset.clone();
        tokio::task::spawn_blocking(move || {
            let mut codec = Codec::new(&input_charset);
            let result = (|| -> Result<()> {
                while let Some(chunk) = input_receive.blocking_recv() {
                    if write_cancel.is_cancelled() {
                        return Ok(());
                    }
                    let bytes = codec.input(&chunk.data)?;
                    let deadline = Instant::now() + Duration::from_secs(5);
                    let mut remaining = bytes.as_slice();
                    while !remaining.is_empty() {
                        if write_cancel.is_cancelled() {
                            return Ok(());
                        }
                        // 逐块检查取消，5秒内无完整交付即结束；不调用可能无限等待的flush。
                        if Instant::now() >= deadline {
                            return Err(SessionError::new(
                                "INPUT_DELIVERY_UNKNOWN",
                                "input",
                                "串口写入超时，可能等待流控信号；未确认输入不会自动重发",
                            ));
                        }
                        match writer.write(remaining) {
                            Ok(0) => return Err(io_error("input")),
                            Ok(count) => remaining = &remaining[count..],
                            Err(error)
                                if matches!(
                                    error.kind(),
                                    ErrorKind::Interrupted | ErrorKind::WouldBlock
                                ) =>
                            {
                                continue;
                            }
                            // 写超时可能已有部分字节进入设备；不能重发同一段。
                            Err(error) if error.kind() == ErrorKind::TimedOut => {
                                return Err(SessionError::new(
                                    "INPUT_DELIVERY_UNKNOWN",
                                    "input",
                                    "串口写入超时，交付状态不确定；未确认输入不会自动重发",
                                ));
                            }
                            Err(_) => return Err(io_error("input")),
                        }
                    }
                }
                Ok(())
            })();
            drop(writer);
            if let Err(error) = result {
                let _ = failure_send.try_send(error);
            }
        });
        context.events.state(SessionState::Connected)?;
        let output = context.output;
        let output_pump = async move {
            let mut codec = Codec::new(&charset);
            while let Some(bytes) = output_receive.recv().await {
                let bytes = codec.output(&bytes, false);
                // 多字节字符尚未完整时不发送空帧。
                for chunk in bytes.chunks(32768) {
                    output.send(Bytes::copy_from_slice(chunk)).await?;
                }
            }
            let tail = codec.output(&[], true);
            if !tail.is_empty() {
                output.send(Bytes::from(tail)).await?;
            }
            Ok::<_, SessionError>(())
        };
        tokio::pin!(output_pump);
        let mut input_send = Some(input_send);
        let mut failure = None;
        loop {
            tokio::select! {
                biased;
                _ = context.cancel.cancelled() => return Err(SessionError::cancelled()),
                error = failure_receive.recv(), if failure.is_none() => {
                    failure = Some(error.unwrap_or_else(|| io_error("serial")));
                    worker_cancel.cancel();
                    input_send.take();
                    // 错误时先停止设备线程，再排空已读取字节；取消仍由独立分支优先处理。
                },
                result = &mut output_pump => { result?; return Err(failure.unwrap_or_else(|| io_error("output"))); },
                chunk = context.input.recv(), if failure.is_none() => {
                    let Some(chunk) = chunk else { return Err(SessionError::cancelled()); };
                    tokio::select! {
                        biased;
                        _ = context.cancel.cancelled() => return Err(SessionError::cancelled()),
                        result = input_send.as_ref().unwrap().send(chunk) => {
                            if result.is_err() {
                                failure = Some(failure_receive.try_recv().unwrap_or_else(|_| io_error("input")));
                                worker_cancel.cancel();
                                input_send.take();
                            }
                        },
                    }
                },
            }
        }
    }
}
