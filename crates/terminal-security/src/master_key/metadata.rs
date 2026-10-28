//! 固定派生参数、认证元数据及其原子发布；不持有数据库生命周期状态。
use super::unavailable;
use crate::SecurityError;
use aes_gcm::{
    Aes256Gcm, KeyInit,
    aead::{Aead, Payload},
};
use argon2::{Algorithm, Argon2, Params, Version};
use getrandom::SysRng;
use rand_core::TryRng;
use serde::{Deserialize, Serialize};
use std::{
    fs::OpenOptions,
    io::{Read, Write},
    path::Path,
};
use zeroize::Zeroizing;

pub(super) const KDF: &str = "argon2id-v19-m65536-t3-p1";
const CHECK: &[u8] = b"termeta/encrypted-vault/v2";

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Header {
    pub(super) format: u32,
    pub(super) id: String,
    pub(super) kdf: String,
    pub(super) salt: [u8; 16],
    pub(super) nonce: [u8; 12],
    pub(super) verifier: Vec<u8>,
    pub(super) initialized: bool,
}
pub(super) fn derive(
    password: &str,
    salt: &[u8; 16],
) -> Result<Zeroizing<[u8; 32]>, SecurityError> {
    let params = Params::new(65536, 3, 1, Some(32)).map_err(|_| unavailable())?;
    let mut key = Zeroizing::new([0; 32]);
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(password.as_bytes(), salt, key.as_mut())
        .map_err(|_| unavailable())?;
    Ok(key)
}
impl Header {
    fn aad(&self) -> Vec<u8> {
        let mut aad = [CHECK, self.id.as_bytes(), self.kdf.as_bytes(), &self.salt].concat();
        aad.push(u8::from(self.initialized));
        aad
    }
    pub(super) fn verify(&self, key: &[u8; 32]) -> Result<(), SecurityError> {
        let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| unavailable())?;
        let clear = cipher
            .decrypt(
                (&self.nonce).into(),
                Payload {
                    msg: &self.verifier,
                    aad: &self.aad(),
                },
            )
            .map_err(|_| SecurityError::Vault("密码不正确或保险库校验数据损坏"))?;
        if clear != CHECK {
            return Err(unavailable());
        }
        Ok(())
    }
    pub(super) fn seal(&mut self, key: &[u8; 32]) -> Result<(), SecurityError> {
        // 完成标记也是认证数据；状态变更必须使用新的 nonce，禁止重用同一 GCM nonce。
        SysRng
            .try_fill_bytes(&mut self.nonce)
            .map_err(|_| unavailable())?;
        self.verifier = Aes256Gcm::new_from_slice(key)
            .map_err(|_| unavailable())?
            .encrypt(
                (&self.nonce).into(),
                Payload {
                    msg: CHECK,
                    aad: &self.aad(),
                },
            )
            .map_err(|_| unavailable())?;
        Ok(())
    }
    pub(super) fn create(password: &str) -> Result<(Self, Zeroizing<[u8; 32]>), SecurityError> {
        let mut header = Self {
            format: 2,
            id: uuid::Uuid::new_v4().to_string(),
            kdf: KDF.into(),
            salt: [0; 16],
            nonce: [0; 12],
            verifier: Vec::new(),
            initialized: false,
        };
        SysRng
            .try_fill_bytes(&mut header.salt)
            .map_err(|_| unavailable())?;
        let key = derive(password, &header.salt)?;
        header.seal(&key)?;
        Ok((header, key))
    }
}
pub(super) fn read_header(path: &Path) -> Result<Option<Header>, SecurityError> {
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(unavailable()),
    };
    let mut bytes = Vec::new();
    file.take(4097)
        .read_to_end(&mut bytes)
        .map_err(|_| unavailable())?;
    if bytes.len() > 4096 {
        return Err(unavailable());
    }
    let header: Header = serde_json::from_slice(&bytes).map_err(|_| unavailable())?;
    if header.format != 2
        || header.kdf != KDF
        || uuid::Uuid::parse_str(&header.id).is_err()
        || header.verifier.len() != CHECK.len() + 16
    {
        return Err(unavailable());
    }
    Ok(Some(header))
}
pub(super) fn write_header(
    path: &Path,
    header: &Header,
    replace: bool,
) -> Result<(), SecurityError> {
    let target = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let result = (|| {
        let mut file = options.open(&target).map_err(|_| unavailable())?;
        file.write_all(&serde_json::to_vec(header).map_err(|_| unavailable())?)
            .map_err(|_| unavailable())?;
        file.sync_all().map_err(|_| unavailable())?;
        if replace {
            std::fs::rename(&target, path).map_err(|_| unavailable())?;
        } else {
            // 首次元数据也原子发布，避免中断写入遗失盐；独占链接禁止覆盖已有保险库。
            std::fs::hard_link(&target, path).map_err(|_| unavailable())?;
            std::fs::remove_file(&target).map_err(|_| unavailable())?;
        }
        #[cfg(unix)]
        if let Some(parent) = path.parent() {
            std::fs::File::open(parent)
                .and_then(|dir| dir.sync_all())
                .map_err(|_| unavailable())?;
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(target);
    }
    result
}
