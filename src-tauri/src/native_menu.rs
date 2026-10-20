use serde::Deserialize;

// 只接受解析后的支持语言，前端不能提交菜单 ID、任意文案或原生动作。
#[derive(Clone, Copy, Deserialize)]
pub enum NativeMenuLocale {
    #[serde(rename = "zh-CN")]
    ZhCn,
    #[serde(rename = "en-US")]
    EnUs,
    #[serde(rename = "zh-TW")]
    ZhTw,
    #[serde(rename = "ja-JP")]
    JaJp,
    #[serde(rename = "ko-KR")]
    KoKr,
}

#[cfg(target_os = "macos")]
impl NativeMenuLocale {
    pub fn labels(self) -> tauri::Result<std::collections::HashMap<String, String>> {
        // 与前端词典共享 JSON 源文件，避免在 Rust 维护另一套译文。
        let source = match self {
            Self::ZhCn => include_str!("../../src/i18n/zh-CN/nativeMenu.json"),
            Self::EnUs => include_str!("../../src/i18n/en-US/nativeMenu.json"),
            Self::ZhTw => include_str!("../../src/i18n/zh-TW/nativeMenu.json"),
            Self::JaJp => include_str!("../../src/i18n/ja-JP/nativeMenu.json"),
            Self::KoKr => include_str!("../../src/i18n/ko-KR/nativeMenu.json"),
        };
        Ok(serde_json::from_str(source)?)
    }
}
