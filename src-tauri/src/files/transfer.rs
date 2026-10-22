use std::sync::Arc;
use terminal_core::CancellationToken;
use terminal_sftp::SftpBrowser;

/// 统一持有本次传输的登记；退出作用域时阻断迟到回调并释放对应登记。
pub(crate) struct TransferRegistration {
    browsers: Vec<Arc<SftpBrowser>>,
    id: String,
    cancel: CancellationToken,
}

impl TransferRegistration {
    pub(crate) fn new(
        id: &str,
        cancel: CancellationToken,
        browsers: impl IntoIterator<Item = Arc<SftpBrowser>>,
    ) -> Self {
        let browsers: Vec<_> = browsers.into_iter().collect();
        for browser in &browsers {
            browser.register_transfer(id, cancel.clone());
        }
        Self {
            browsers,
            id: id.into(),
            cancel,
        }
    }
}

impl Drop for TransferRegistration {
    fn drop(&mut self) {
        // 原生拖放结束后即使登记已撤销，迟到的文件承诺也不得继续读取远端。
        self.cancel.cancel();
        for browser in &self.browsers {
            browser.unregister_transfer(&self.id);
        }
    }
}
