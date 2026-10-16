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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gbk_keeps_split_characters_and_control_bytes() {
        let mut codec = Codec::new("GBK");
        assert!(codec.output(&[0xd6], false).is_empty());
        assert_eq!(
            codec.output(&[0xd0, 0x1b, b'[', b'm'], false),
            "中\x1b[m".as_bytes()
        );
        assert!(codec.input(&[0xe4, 0xb8]).unwrap().is_empty());
        assert_eq!(codec.input(&[0xad, b'\r']).unwrap(), [0xd6, 0xd0, b'\r']);
    }

    #[test]
    fn rejects_unrepresentable_input_and_flushes_truncated_output() {
        let mut codec = Codec::new("windows-1252");
        assert!(codec.input("中".as_bytes()).is_err());
        let mut codec = Codec::new("GBK");
        assert!(codec.output(&[0xd6], false).is_empty());
        assert_eq!(codec.output(&[], true), "�".as_bytes());
    }

    #[test]
    fn utf8_replaces_invalid_output_and_keeps_control_bytes() {
        let mut codec = Codec::new("UTF-8");
        assert_eq!(
            codec.output(b"\0GP\xff@\xe4\xb8K\x80S\xfe\x07\x1b[31m\r\n", false),
            "\0GP�@�K�S�\x07\x1b[31m\r\n".as_bytes()
        );
    }

    #[test]
    fn utf8_output_is_independent_of_read_boundaries() {
        // 混合有效多字节、非法起始/续字节和控制序列，验证任意读取边界。
        let bytes = b"GP\xff\xe4\xb8\xad\xf0\x9f\x98\x80\xe4\xb8K\xed\xa0\x80\x1b[0m\0\xe2\x82";
        let expected = String::from_utf8_lossy(bytes);
        for split in 0..=bytes.len() {
            let mut codec = Codec::new("UTF-8");
            let mut output = codec.output(&bytes[..split], false);
            output.extend(codec.output(&bytes[split..], false));
            output.extend(codec.output(&[], true));
            assert_eq!(output, expected.as_bytes(), "读取边界：{split}");
        }
        let mut codec = Codec::new("UTF-8");
        let mut output = Vec::new();
        for byte in bytes {
            output.extend(codec.output(&[*byte], false));
        }
        output.extend(codec.output(&[], true));
        assert_eq!(output, expected.as_bytes());
    }

    #[test]
    fn utf8_flushes_incomplete_output_only_at_end() {
        let mut codec = Codec::new("UTF-8");
        assert!(codec.output(&[0xe4], false).is_empty());
        assert!(codec.output(&[0xb8], false).is_empty());
        assert!(codec.output(&[], false).is_empty());
        assert_eq!(codec.output(&[], true), "�".as_bytes());
    }

    #[test]
    fn utf8_input_preserves_bytes() {
        let mut codec = Codec::new("UTF-8");
        let bytes = [0, 0xff, 0x1b, b'\r'];
        assert_eq!(codec.input(&bytes).unwrap(), bytes);
    }
}
