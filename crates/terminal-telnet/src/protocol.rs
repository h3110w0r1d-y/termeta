//! RFC 854/855、856、857、858、1091、1073 的受界流式适配。
use terminal_core::{Result, SessionError, TerminalSize};

const IAC: u8 = 255;
const DONT: u8 = 254;
const DO: u8 = 253;
const WONT: u8 = 252;
const WILL: u8 = 251;
const SB: u8 = 250;
const SE: u8 = 240;
const BINARY: u8 = 0;
const ECHO: u8 = 1;
const SGA: u8 = 3;
const TTYPE: u8 = 24;
const NAWS: u8 = 31;
const MAX_SUBNEGOTIATION: usize = 4096;

#[derive(Clone, Copy, Default, PartialEq)]
enum OptionState {
    #[default]
    No,
    WantYes,
    Yes,
}
#[derive(Default)]
enum ParseState {
    #[default]
    Data,
    Command,
    Option(u8),
    Sub,
    SubCommand,
}

pub struct Protocol {
    state: ParseState,
    local: [OptionState; 256],
    remote: [OptionState; 256],
    sub: Vec<u8>,
    pending_cr: bool,
    input_cr: bool,
    size: TerminalSize,
}
impl Protocol {
    pub fn new(size: TerminalSize) -> Self {
        Self {
            state: ParseState::Data,
            local: [OptionState::No; 256],
            remote: [OptionState::No; 256],
            sub: Vec::new(),
            pending_cr: false,
            input_cr: false,
            size,
        }
    }
    pub fn initial(&mut self) -> Vec<u8> {
        let mut wire = Vec::new();
        for option in [BINARY, SGA, TTYPE, NAWS] {
            self.local[option as usize] = OptionState::WantYes;
            wire.extend_from_slice(&[IAC, WILL, option]);
        }
        for option in [BINARY, ECHO, SGA] {
            self.remote[option as usize] = OptionState::WantYes;
            wire.extend_from_slice(&[IAC, DO, option]);
        }
        wire
    }
    pub fn resize(&mut self, size: TerminalSize) -> Vec<u8> {
        self.size = size;
        if self.local[NAWS as usize] == OptionState::Yes {
            self.window_size()
        } else {
            Vec::new()
        }
    }
    fn window_size(&self) -> Vec<u8> {
        let mut payload = vec![NAWS];
        payload.extend_from_slice(&self.size.cols.to_be_bytes());
        payload.extend_from_slice(&self.size.rows.to_be_bytes());
        subnegotiation(&payload)
    }
    /// xterm Enter 为 CR，NVT 下发 CR LF；跨输入块的 CR LF 只发送一次。
    /// BINARY 协商成功后保留原始控制字节，所有模式均转义 IAC。
    pub fn input(&mut self, bytes: &[u8]) -> Vec<u8> {
        let binary = self.local[BINARY as usize] == OptionState::Yes;
        let mut wire = Vec::with_capacity(bytes.len() * 2);
        for &byte in bytes {
            let follows_cr = std::mem::take(&mut self.input_cr);
            if !binary && byte == b'\n' && follows_cr {
                continue;
            }
            if !binary && matches!(byte, b'\r' | b'\n') {
                wire.extend_from_slice(b"\r\n");
                self.input_cr = byte == b'\r';
            } else {
                escape(byte, &mut wire);
            }
        }
        wire
    }
    /// 解码状态跨 TCP 分片保留，控制序列不进入终端正文。
    pub fn receive(&mut self, bytes: &[u8]) -> Result<(Vec<u8>, Vec<u8>)> {
        let mut data = Vec::with_capacity(bytes.len());
        let mut wire = Vec::new();
        for &byte in bytes {
            match std::mem::take(&mut self.state) {
                ParseState::Data => {
                    if byte == IAC {
                        self.state = ParseState::Command;
                    } else {
                        self.data(byte, &mut data);
                    }
                }
                ParseState::Command => match byte {
                    IAC => self.data(IAC, &mut data),
                    WILL | WONT | DO | DONT => self.state = ParseState::Option(byte),
                    SB => {
                        self.sub.clear();
                        self.state = ParseState::Sub;
                    }
                    _ => {} // NOP/GA/单字节命令不显示为正文。
                },
                ParseState::Option(command) => self.option(command, byte, &mut wire),
                ParseState::Sub => {
                    if byte == IAC {
                        self.state = ParseState::SubCommand;
                    } else {
                        self.sub_byte(byte)?;
                        self.state = ParseState::Sub;
                    }
                }
                ParseState::SubCommand => match byte {
                    SE => {
                        if self.local[TTYPE as usize] == OptionState::Yes && self.sub == [TTYPE, 1]
                        {
                            wire.extend(subnegotiation(b"\x18\x00xterm-256color"));
                        }
                        self.sub.clear();
                    }
                    IAC => {
                        self.sub_byte(IAC)?;
                        self.state = ParseState::Sub;
                    }
                    _ => return Err(protocol_error()),
                },
            }
        }
        Ok((data, wire))
    }
    fn sub_byte(&mut self, byte: u8) -> Result<()> {
        if self.sub.len() >= MAX_SUBNEGOTIATION {
            return Err(protocol_error());
        }
        self.sub.push(byte);
        Ok(())
    }
    fn data(&mut self, byte: u8, data: &mut Vec<u8>) {
        // CR 立即输出，只跨分片抑制其后的 NUL；CR LF 自然保留。
        if self.pending_cr {
            self.pending_cr = false;
            if byte == 0 {
                return;
            }
        }
        self.pending_cr = byte == b'\r' && self.remote[BINARY as usize] != OptionState::Yes;
        data.push(byte);
    }
    fn option(&mut self, command: u8, option: u8, wire: &mut Vec<u8>) {
        let local = command == DO || command == DONT;
        let positive = command == DO || command == WILL;
        let accepted = if local {
            matches!(option, BINARY | SGA | TTYPE | NAWS)
        } else {
            matches!(option, BINARY | ECHO | SGA)
        };
        let states = if local {
            &mut self.local
        } else {
            &mut self.remote
        };
        let previous = states[option as usize];
        let yes = if local { WILL } else { DO };
        let no = if local { WONT } else { DONT };
        if positive && accepted {
            states[option as usize] = OptionState::Yes;
            if previous == OptionState::No {
                wire.extend_from_slice(&[IAC, yes, option]);
            }
            if local && option == NAWS && previous != OptionState::Yes {
                wire.extend(self.window_size());
            }
        } else {
            states[option as usize] = OptionState::No;
            if positive || previous == OptionState::Yes {
                wire.extend_from_slice(&[IAC, no, option]);
            }
        }
    }
}
fn escape(byte: u8, wire: &mut Vec<u8>) {
    wire.push(byte);
    if byte == IAC {
        wire.push(IAC);
    }
}
fn subnegotiation(payload: &[u8]) -> Vec<u8> {
    let mut wire = vec![IAC, SB];
    for &byte in payload {
        escape(byte, &mut wire);
    }
    wire.extend_from_slice(&[IAC, SE]);
    wire
}
fn protocol_error() -> SessionError {
    SessionError::new(
        "TELNET_PROTOCOL_ERROR",
        "telnet",
        "Telnet 子协商格式无效或超过 4096 字节限制",
    )
}
