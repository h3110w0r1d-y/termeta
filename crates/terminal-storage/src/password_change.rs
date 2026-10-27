use crate::*;
use std::{fs::OpenOptions, io::Write};

const MARKER: &[u8] = b"termeta-password-change-v1\n";
const FILES: &[&str] = &[
    "original.sqlite3",
    "original.vault.json",
    "next.sqlite3",
    "next.sqlite3-journal",
    "next.sqlite3-wal",
    "next.sqlite3-shm",
    "next.vault.json",
    "displaced.sqlite3",
    "displaced.vault.json",
    "restore.sqlite3",
    "restore.vault.json",
    "prepared.tmp",
    "committed.tmp",
    "rolled-back.tmp",
    "prepared",
    "committed",
    "rolled-back",
];

pub struct PasswordChangeOutcome {
    pub cleanup_pending: bool,
}

#[derive(Clone)]
struct Journal {
    database: PathBuf,
    directory: PathBuf,
}
impl Journal {
    fn new(database: &Path) -> Self {
        Self {
            database: database.to_owned(),
            directory: database.with_extension("password-change"),
        }
    }
    fn metadata(&self) -> PathBuf {
        self.database.with_extension("vault.json")
    }
    fn file(&self, name: &str) -> PathBuf {
        self.directory.join(name)
    }
    fn create(&self, metadata: &[u8]) -> Result<()> {
        let mut builder = std::fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(&self.directory)?;
        sync_parent(&self.directory)?;
        copy_new(&self.database, &self.file("original.sqlite3"))?;
        copy_new(&self.metadata(), &self.file("original.vault.json"))?;
        copy_new(&self.database, &self.file("next.sqlite3"))?;
        write_new(&self.file("next.vault.json"), metadata)?;
        sync_directory(&self.directory)
    }
    fn mark(&self, name: &str) -> Result<()> {
        let temporary = self.file(&format!("{name}.tmp"));
        remove_file(&temporary)?;
        write_new(&temporary, MARKER)?;
        std::fs::rename(temporary, self.file(name))?;
        sync_directory(&self.directory)
    }
    fn marked(&self, name: &str) -> Result<bool> {
        let path = self.file(name);
        if !path.try_exists()? {
            return Ok(false);
        }
        regular_file(&path)?;
        if std::fs::metadata(&path)?.len() != MARKER.len() as u64 || std::fs::read(path)? != MARKER
        {
            return Err(StorageError::Validation(
                "密码修改记录损坏，请保留数据库与修改记录后重试",
            ));
        }
        Ok(true)
    }
    fn install(&self) -> Result<()> {
        // 每一步都可中断；prepared 已持久化后，启动恢复才能替换原库。
        for (live, displaced, next) in [
            (self.database.clone(), "displaced.sqlite3", "next.sqlite3"),
            (self.metadata(), "displaced.vault.json", "next.vault.json"),
        ] {
            std::fs::rename(&live, self.file(displaced))?;
            std::fs::rename(self.file(next), live)?;
        }
        sync_parent(&self.database)?;
        sync_directory(&self.directory)
    }
    fn restore(&self) -> Result<()> {
        regular_file(&self.file("original.sqlite3"))?;
        regular_file(&self.file("original.vault.json"))?;
        // 所有连接已关闭，原快照已包含旧 WAL；不能把新密钥的侧文件留给原库。
        for suffix in ["-wal", "-shm", "-journal"] {
            remove_file(&self.database.with_file_name(format!(
                "{}{suffix}",
                self.database.file_name().unwrap().to_string_lossy()
            )))?;
        }
        for (original, temporary, live) in [
            ("original.sqlite3", "restore.sqlite3", self.database.clone()),
            ("original.vault.json", "restore.vault.json", self.metadata()),
        ] {
            // 原始副本始终保留，恢复自身再次中断时仍能重复执行。
            remove_file(&self.file(temporary))?;
            remove_file(&live)?;
            copy_new(&self.file(original), &self.file(temporary))?;
            std::fs::rename(self.file(temporary), live)?;
        }
        sync_parent(&self.database)?;
        self.mark("rolled-back")
    }
    fn cleanup(&self) -> Result<()> {
        // 只清理本协议的固定文件，且最后删除状态标记，避免再次恢复覆盖后续写入。
        for name in FILES
            .iter()
            .filter(|name| !matches!(**name, "committed" | "rolled-back"))
        {
            remove_file(&self.file(name))?;
        }
        // prepared 的删除先落盘，再删除终态标记；断电不能让 prepared 单独重新出现。
        sync_directory(&self.directory)?;
        remove_file(&self.file("committed"))?;
        remove_file(&self.file("rolled-back"))?;
        sync_directory(&self.directory)?;
        std::fs::remove_dir(&self.directory)?;
        sync_parent(&self.directory)
    }
    fn recover(&self) -> Result<bool> {
        if !self.directory.try_exists()? {
            return Ok(false);
        }
        let attributes = std::fs::symlink_metadata(&self.directory)?;
        if !attributes.is_dir() || attributes.file_type().is_symlink() {
            return Err(StorageError::Validation(
                "密码修改记录路径无效，请保留数据后重试",
            ));
        }
        let committed = self.marked("committed")?;
        let restored = self.marked("rolled-back")?;
        if committed && restored {
            return Err(StorageError::Validation(
                "密码修改记录状态冲突，请保留数据后重试",
            ));
        }
        if !committed && !restored && self.marked("prepared")? {
            self.restore()?;
        }
        // 先保留所有副本，只有恢复后的数据库通过密钥/完整性/迁移校验才清理。
        Ok(committed)
    }
}

fn regular_file(path: &Path) -> Result<()> {
    let attributes = std::fs::symlink_metadata(path)?;
    if !attributes.is_file() || attributes.file_type().is_symlink() {
        return Err(StorageError::Validation(
            "保险库文件类型无效，请保留数据后重试",
        ));
    }
    Ok(())
}
fn new_file(path: &Path) -> Result<std::fs::File> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    Ok(options.open(path)?)
}
fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = new_file(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}
fn copy_new(source: &Path, target: &Path) -> Result<()> {
    regular_file(source)?;
    let mut source = std::fs::File::open(source)?;
    let mut target = new_file(target)?;
    std::io::copy(&mut source, &mut target)?;
    target.sync_all()?;
    Ok(())
}
fn remove_file(path: &Path) -> Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}
fn sync_directory(path: &Path) -> Result<()> {
    #[cfg(unix)]
    std::fs::File::open(path)?.sync_all()?;
    let _ = path;
    Ok(())
}
fn sync_parent(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        sync_directory(parent)?;
    }
    Ok(())
}
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T> + Send + 'static,
) -> Result<T> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|_| StorageError::Validation("保险库文件工作器不可用，请保留数据后重试"))?
}

impl SqliteStore {
    /// 必须在读取保险库元数据前恢复；已解锁后绝不重新应用旧快照。
    pub async fn recover_password_change(&self) -> Result<()> {
        let slot = self.pool.write().await;
        if slot.is_some() {
            return Ok(());
        }
        let journal = Journal::new(&self.path);
        blocking(move || journal.recover().map(|_| ())).await
    }
    /// 仅在新组合或恢复后的原组合通过校验后清理，失败留下终态标记供下次重试。
    pub async fn finish_password_change_recovery(&self) -> Result<bool> {
        let slot = self.pool.write().await;
        if slot.is_none() {
            return Err(StorageError::Locked);
        }
        let journal = Journal::new(&self.path);
        blocking(move || {
            if !journal.directory.try_exists()? {
                return Ok(false);
            }
            Ok(journal.cleanup().is_err())
        })
        .await
    }
    async fn checked_pool(
        path: &Path,
        key: &[u8; 32],
        journal: SqliteJournalMode,
    ) -> Result<SqlitePool> {
        let pool = SqlitePoolOptions::new()
            .max_connections(if matches!(journal, SqliteJournalMode::Delete) {
                1
            } else {
                2
            })
            .connect_with(Self::connect_options(path, key, journal))
            .await?;
        if let Err(error) = Self::validate(&pool, path).await {
            pool.close().await;
            return Err(error);
        }
        let violations = sqlx::query("PRAGMA foreign_key_check")
            .fetch_all(&pool)
            .await;
        match violations {
            Ok(rows) if rows.is_empty() => {}
            result => {
                pool.close().await;
                return match result {
                    Err(error) => Err(error.into()),
                    _ => Err(StorageError::Validation("数据库外键完整性校验失败")),
                };
            }
        }
        Ok(pool)
    }
    async fn prepare_password_change(
        &self,
        journal: &Journal,
        old_key: &[u8; 32],
        new_key: &[u8; 32],
        metadata: Vec<u8>,
    ) -> Result<SqlitePool> {
        // 切至 DELETE 并关闭所有连接，让最近的 WAL 写入完整进入加密主文件。
        let source = Self::checked_pool(&self.path, old_key, SqliteJournalMode::Delete).await?;
        source.close().await;
        let files = journal.clone();
        blocking(move || files.create(&metadata)).await?;
        let target = journal.file("next.sqlite3");
        let staged = Self::checked_pool(&target, old_key, SqliteJournalMode::Delete).await?;
        let encoded = zeroize::Zeroizing::new(hex::encode(new_key));
        let statement =
            zeroize::Zeroizing::new(format!("PRAGMA rekey = \"x'{}'\"", encoded.as_str()));
        // 只在独占的加密副本上改密钥；连接禁用 SQL 日志并在完成后立即关闭。
        let rekeyed = sqlx::query(&statement).execute(&staged).await;
        staged.close().await;
        rekeyed?;
        let verified = Self::checked_pool(&target, new_key, SqliteJournalMode::Delete).await?;
        verified.close().await;
        let files = journal.clone();
        blocking(move || {
            // Windows 的 FlushFileBuffers 需要可写句柄。
            OpenOptions::new()
                .read(true)
                .write(true)
                .open(files.file("next.sqlite3"))?
                .sync_all()?;
            files.mark("prepared")?;
            files.install()
        })
        .await?;
        // 重新连接只验证，不执行启动迁移和历史中断标记，不影响活动终端。
        let candidate = Self::checked_pool(&self.path, new_key, SqliteJournalMode::Wal).await?;
        let files = journal.clone();
        if let Err(error) = blocking(move || files.mark("committed")).await {
            candidate.close().await;
            return Err(error);
        }
        Ok(candidate)
    }
    /// 调用方串行协调凭据操作；数据库读请求等待切换，输出与取消通道不经过此门闩。
    pub async fn change_key(
        &self,
        old_key: &[u8; 32],
        new_key: &[u8; 32],
        metadata: Vec<u8>,
    ) -> Result<PasswordChangeOutcome> {
        let mut slot = self.pool.write().await;
        let journal = Journal::new(&self.path);
        if journal.directory.try_exists()? {
            return Err(StorageError::Validation(
                "上次密码修改记录尚未清理，请退出应用后重试",
            ));
        }
        let pool = slot.take().ok_or(StorageError::Locked)?;
        self.unlocked.store(false, Ordering::Release);
        pool.close().await;
        match self
            .prepare_password_change(&journal, old_key, new_key, metadata)
            .await
        {
            Ok(pool) => {
                *slot = Some(pool);
                self.unlocked.store(true, Ordering::Release);
                // 提交后的清理错误不再伪报改密失败，原副本留待下次启动清理。
                let cleanup_pending = blocking(move || Ok(journal.cleanup().is_err()))
                    .await
                    .unwrap_or(true);
                Ok(PasswordChangeOutcome { cleanup_pending })
            }
            Err(error) => {
                let files = journal.clone();
                let recovered = blocking(move || files.recover()).await;
                if let Ok(committed) = recovered {
                    let key = if committed { new_key } else { old_key };
                    if let Ok(pool) =
                        Self::checked_pool(&self.path, key, SqliteJournalMode::Wal).await
                    {
                        *slot = Some(pool);
                        self.unlocked.store(true, Ordering::Release);
                        let cleanup_pending = blocking(move || {
                            if !journal.directory.try_exists()? {
                                return Ok(false);
                            }
                            Ok(journal.cleanup().is_err())
                        })
                        .await
                        .unwrap_or(true);
                        return if committed {
                            Ok(PasswordChangeOutcome { cleanup_pending })
                        } else {
                            Err(error)
                        };
                    }
                }
                Err(StorageError::Validation(
                    "密码修改未能完成恢复，请退出应用后重试；请保留数据库、vault 元数据与 password-change 目录",
                ))
            }
        }
    }
}
