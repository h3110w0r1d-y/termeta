use serde::Deserialize;

// 只接受解析后的支持语言，前端不能提交菜单 ID、任意文案或原生动作。
#[derive(Clone, Copy, Deserialize)]
pub enum NativeMenuLocale {
    #[serde(rename = "zh-CN")]
    ZhCn,
    #[serde(rename = "en-US")]
    EnUs,
}

#[cfg(target_os = "macos")]
impl NativeMenuLocale {
    pub fn labels(self) -> tauri::Result<std::collections::HashMap<String, String>> {
        // 与前端词典共享 JSON 源文件，避免在 Rust 维护另一套译文。
        let source = match self {
            Self::ZhCn => include_str!("../../src/i18n/messages/zh-CN/nativeMenu.json"),
            Self::EnUs => include_str!("../../src/i18n/messages/en-US/nativeMenu.json"),
        };
        Ok(serde_json::from_str(source)?)
    }
}
