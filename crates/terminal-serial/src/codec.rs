use encoding_rs::{CoderResult, Decoder, Encoding, UTF_8};
use terminal_core::{Result, SessionError};

pub(crate) struct Codec {
    encoding: &'static Encoding,
    decoder: Decoder,
    input: Decoder,
}
impl Codec {
    pub(crate) fn new(charset: &str) -> Self {
        // 字符集由核心白名单校验，禁止UTF-16等破坏终端控制字节的编码。
        let encoding = Encoding::for_label(charset.as_bytes()).expect("已校验的串口字符集");
        Self {
            encoding,
            decoder: encoding.new_decoder_without_bom_handling(),
            input: UTF_8.new_decoder_without_bom_handling(),
        }
    }
    pub(crate) fn output(&mut self, bytes: &[u8], last: bool) -> Vec<u8> {
        // 包括UTF-8在内统一流式解码，避免xterm字节解码器跳过非法设备字节。
        // 保留跨读取块的多字节序列与终端控制字节，非法序列显示替换字符。
        let mut text =
            String::with_capacity(self.decoder.max_utf8_buffer_length(bytes.len()).unwrap());
        let (result, read, _) = self.decoder.decode_to_string(bytes, &mut text, last);
        debug_assert_eq!(result, CoderResult::InputEmpty);
        debug_assert_eq!(read, bytes.len());
        text.into_bytes()
    }
    pub(crate) fn input(&mut self, bytes: &[u8]) -> Result<Vec<u8>> {
        if self.encoding == UTF_8 {
            return Ok(bytes.to_vec());
        }
        // IPC切块可能位于UTF-8字符中间；各串口拥有独立解码状态。
        let mut text =
            String::with_capacity(self.input.max_utf8_buffer_length(bytes.len()).unwrap());
        let (result, read, invalid) = self.input.decode_to_string(bytes, &mut text, false);
        debug_assert_eq!(result, CoderResult::InputEmpty);
        debug_assert_eq!(read, bytes.len());
        let (encoded, _, unmappable) = self.encoding.encode(&text);
        if invalid || unmappable {
            return Err(SessionError::new(
                "CHARSET_UNMAPPABLE",
                "input",
                "输入包含当前串口字符集无法表示的字符，请更换字符集后重新连接",
            ));
        }
        Ok(encoded.into_owned())
    }
}
