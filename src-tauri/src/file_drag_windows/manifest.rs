//! Windows 文件描述符采用父目录在前的相对路径清单，空目录也必须单独保留。
use std::collections::HashSet;
use terminal_core::{CancellationToken, Result, SessionError};
use terminal_files::{child_path, file_error, validate_name};
use terminal_sftp::SftpBrowser;

pub(super) const MAX_ENTRIES: usize = 10000;
const MAX_DEPTH: usize = 64;

pub(super) struct Root {
    pub name: String,
    pub remote: String,
    pub directory: bool,
}

pub(super) struct Entry {
    pub relative: String,
    pub remote: String,
    pub directory: bool,
    pub size: u64,
    pub modified: Option<u32>,
}

pub(super) struct Manifest {
    pub entries: Vec<Entry>,
}

/// 拒绝 Windows 会改写、解释为设备或折叠成同名目标的名称，不静默重命名。
pub(super) fn validate_component(name: &str) -> Result<()> {
    validate_name(name)?;
    let stem = name
        .split('.')
        .next()
        .unwrap_or_default()
        .trim_end_matches(' ')
        .to_uppercase();
    let device = matches!(
        stem.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) || ["COM", "LPT"].iter().any(|prefix| {
        stem.strip_prefix(prefix).is_some_and(|suffix| {
            matches!(
                suffix,
                "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
            )
        })
    });
    if device
        || name.ends_with([' ', '.'])
        || name
            .chars()
            .any(|c| matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*'))
    {
        return Err(file_error(
            "拖出条目包含 Windows 不支持的文件名，请先重命名",
        ));
    }
    Ok(())
}

pub(super) fn validate_relative(relative: &str) -> Result<()> {
    for component in relative.split('\\') {
        validate_component(component)?;
    }
    if relative.encode_utf16().count() >= 260 {
        return Err(file_error(
            "拖出目录中的相对路径超过 Windows 文件描述符长度限制",
        ));
    }
    Ok(())
}

pub(super) async fn scan(
    browser: &SftpBrowser,
    roots: &[Root],
    cancel: &CancellationToken,
) -> Result<Manifest> {
    let mut pending: Vec<_> = roots
        .iter()
        .rev()
        .map(|root| {
            (
                root.remote.clone(),
                root.name.clone(),
                root.directory,
                0usize,
            )
        })
        .collect();
    let mut discovered = pending.len();
    let mut entries = Vec::new();
    let mut names = HashSet::new();
    while let Some((remote, relative, directory, depth)) = pending.pop() {
        if cancel.is_cancelled() {
            return Err(SessionError::cancelled());
        }
        if depth > MAX_DEPTH {
            return Err(file_error("拖出目录超过 64 层限制"));
        }
        validate_relative(&relative)?;
        // 大写键采用保守冲突检查，避免大小写不同的远端条目在 Windows 合并或覆盖。
        if !names.insert(relative.to_uppercase()) {
            return Err(file_error(
                "拖出目录包含 Windows 无法区分的同名条目，请先重命名",
            ));
        }
        let attrs = browser.metadata(&remote).await?;
        if (directory && !attrs.file_type().is_dir())
            || (!directory && !attrs.file_type().is_file())
        {
            return Err(file_error("拖出源类型已变化，不传输链接或特殊文件"));
        }
        let size = if directory {
            0
        } else {
            attrs
                .size
                .ok_or_else(|| file_error("远端未提供拖出文件大小"))?
        };
        entries.push(Entry {
            relative: relative.clone(),
            remote: remote.clone(),
            directory,
            size,
            modified: attrs.mtime,
        });
        if directory {
            // 复用独立扫描句柄及其取消收尾，不改变界面分页游标。
            let children = browser
                .scan_directory(&remote, MAX_ENTRIES.saturating_sub(discovered), cancel)
                .await?;
            discovered += children.len();
            if discovered > MAX_ENTRIES {
                return Err(file_error("拖出批次超过 10000 项限制"));
            }
            for child in children.into_iter().rev() {
                pending.push((
                    child_path(&remote, &child.name)?,
                    format!("{relative}\\{}", child.name),
                    child.kind == "directory",
                    depth + 1,
                ));
            }
        }
    }
    if cancel.is_cancelled() {
        return Err(SessionError::cancelled());
    }
    Ok(Manifest { entries })
}
