//! 本地PTY驱动：阻塞系统调用留在专用线程，输出复用核心有界队列。
use async_trait::async_trait;
use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use std::{
    io::{Read, Write},
    sync::{Arc, Mutex},
    time::Duration,
};
use terminal_core::*;

pub struct LocalFactory;
fn io_error() -> SessionError {
    SessionError::new("IO_FAILED", "local", "本地终端操作失败")
}
fn pty_size(size: TerminalSize) -> PtySize {
    PtySize {
        rows: size.rows,
        cols: size.cols,
        pixel_width: 0,
        pixel_height: 0,
    }
}
struct Process {
    child: Option<Box<dyn Child + Send + Sync>>,
    master: Box<dyn MasterPty + Send>,
}
impl Drop for Process {
    fn drop(&mut self) {
        // 会话future被取消或初始化结果被丢弃时，同样终止PTY所属进程。
        if let Some(mut child) = self.child.take() {
            #[cfg(unix)]
            if child.try_wait().ok().flatten().is_none() {
                if let Some(pid) = child.process_id() {
                    // portable-pty以setsid创建独立会话；结束初始进程组并确保Shell可回收。
                    unsafe {
                        if let Some(fd) = self.master.as_raw_fd() {
                            let foreground = libc::tcgetpgrp(fd);
                            if foreground > 0 && foreground != pid as i32 {
                                libc::kill(-foreground, libc::SIGKILL);
                            }
                        }
                        libc::kill(-(pid as i32), libc::SIGKILL);
                        libc::kill(pid as i32, libc::SIGKILL);
                    }
                }
            }
            #[cfg(windows)]
            let _ = child.kill();
            // 回收退出状态，避免Unix子进程变成僵尸；等待不占用异步执行器。
            std::thread::spawn(move || {
                let _ = child.wait();
            });
        }
    }
}
#[async_trait]
impl SessionFactory for LocalFactory {
    fn kind(&self) -> &'static str {
        "local"
    }
    fn capabilities(&self) -> SessionCapabilities {
        SessionCapabilities {
            resize: true,
            pty: true,
            binary_output: true,
            serial_control: false,
        }
    }
    fn validate(&self, config: &SessionConfig) -> Result<()> {
        if matches!(config, SessionConfig::Local) {
            Ok(())
        } else {
            Err(SessionError::new(
                "UNSUPPORTED_KIND",
                "opening",
                "本地工厂仅接受本地配置",
            ))
        }
    }
    async fn run(&self, config: SessionConfig, mut context: DriverContext) -> Result<SessionEnd> {
        self.validate(&config)?;
        context.events.state(SessionState::Opening)?;
        let size = context.size;
        let (process, mut reader, mut writer) = tokio::task::spawn_blocking(move || {
            let pair = native_pty_system()
                .openpty(pty_size(size))
                .map_err(|_| io_error())?;
            let shell = if cfg!(windows) {
                std::env::var_os("COMSPEC").unwrap_or_else(|| "cmd.exe".into())
            } else {
                std::env::var_os("SHELL")
                    .filter(|value| !value.is_empty())
                    .unwrap_or_else(|| "/bin/sh".into())
            };
            let mut command = CommandBuilder::new(shell);
            if !cfg!(windows) {
                command.arg("-l");
            }
            if let Some(home) = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
            {
                command.cwd(home);
            }
            command.env("TERM", "xterm-256color");
            command.env("COLORTERM", "truecolor");
            let reader = pair.master.try_clone_reader().map_err(|_| io_error())?;
            let writer = pair.master.take_writer().map_err(|_| io_error())?;
            let child = pair.slave.spawn_command(command).map_err(|_| io_error())?;
            drop(pair.slave);
            Ok::<_, SessionError>((
                Process {
                    child: Some(child),
                    master: pair.master,
                },
                reader,
                writer,
            ))
        })
        .await
        .map_err(|_| io_error())??;
        let process = Arc::new(Mutex::new(process));
        // 关闭future会释放最后一个进程持有者；读写线程不持有进程，防止相互等待。
        let (send, mut receive) = tokio::sync::mpsc::channel::<Result<Vec<u8>>>(4);
        tokio::task::spawn_blocking(move || {
            let mut buffer = [0; 32768];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(count) => {
                        if send.blocking_send(Ok(buffer[..count].to_vec())).is_err() {
                            break;
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(_) => {
                        let _ = send.blocking_send(Err(io_error()));
                        break;
                    }
                }
            }
        });
        let (input_send, mut input_receive) = tokio::sync::mpsc::channel::<InputChunk>(1);
        let (failure_send, failure_receive) = tokio::sync::oneshot::channel();
        tokio::task::spawn_blocking(move || {
            while let Some(chunk) = input_receive.blocking_recv() {
                if writer
                    .write_all(&chunk.data)
                    .and_then(|_| writer.flush())
                    .is_err()
                {
                    let _ = failure_send.send(());
                    break;
                }
            }
        });
        let failure = async move {
            if failure_receive.await.is_err() {
                std::future::pending::<()>().await;
            }
        };
        tokio::pin!(failure);
        context.events.state(SessionState::Connected)?;
        let mut poll = tokio::time::interval(Duration::from_millis(50));
        let output = context.output;
        let output_pump = async move {
            while let Some(chunk) = receive.recv().await {
                match chunk {
                    Ok(bytes) => output.send(Bytes::from(bytes)).await?,
                    // Unix PTY退出可能报告EIO；进程退出状态由独立分支读取。
                    Err(_) => break,
                }
            }
            Ok::<_, SessionError>(())
        };
        tokio::pin!(output_pump);
        let mut output_closed = false;
        let mut exit_code = None;
        loop {
            tokio::select! {
                biased;
                _ = context.cancel.cancelled() => return Err(SessionError::cancelled()),
                _ = &mut failure => return Err(io_error()),
                chunk = context.input.recv() => {
                    let Some(chunk) = chunk else { return Err(SessionError::cancelled()); };
                    tokio::select! {
                        _ = context.cancel.cancelled() => return Err(SessionError::cancelled()),
                        result = input_send.send(chunk) => result.map_err(|_| io_error())?,
                    }
                },
                result = context.resize.changed() => {
                    result.map_err(|_| io_error())?;
                    let size = *context.resize.borrow_and_update();
                    let process = process.clone();
                    tokio::task::spawn_blocking(move || process.lock().unwrap().master.resize(pty_size(size)).map_err(|_| io_error()))
                        .await.map_err(|_| io_error())??;
                },
                result = &mut output_pump, if !output_closed => {
                    result?;
                    output_closed = true;
                },
                _ = poll.tick() => {
                    if exit_code.is_none() {
                        let process = process.clone();
                        let status = tokio::task::spawn_blocking(move || process.lock().unwrap().child.as_mut().unwrap().try_wait().map_err(|_| io_error()))
                            .await.map_err(|_| io_error())??;
                        if let Some(status) = status { exit_code = Some(status.exit_code()); }
                    }
                },
            }
            if output_closed && let Some(code) = exit_code {
                return Ok(SessionEnd {
                    exit_code: Some(code),
                    exit_signal: None,
                });
            }
        }
    }
}
