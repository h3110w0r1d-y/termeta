//! 只读 COM 流按需读取 SFTP，定位/克隆不共享游标，不把整个文件放入内存。
use super::transfer::{Remote, Transfer};
use std::{
    ffi::c_void,
    mem::ManuallyDrop,
    sync::{Arc, Mutex, atomic::Ordering},
};
use windows::{
    Win32::{
        Foundation::{
            E_ABORT, E_POINTER, S_FALSE, S_OK, STG_E_ACCESSDENIED, STG_E_INVALIDFUNCTION,
            STG_E_LOCKVIOLATION, STG_E_READFAULT,
        },
        System::Com::{
            CoTaskMemAlloc, ISequentialStream_Impl, IStream, IStream_Impl, LOCKTYPE, STATFLAG,
            STATFLAG_NONAME, STATSTG, STGC, STGM_READ, STGMEDIUM, STGMEDIUM_0, STGTY_STREAM,
            STREAM_SEEK, STREAM_SEEK_CUR, STREAM_SEEK_END, STREAM_SEEK_SET, TYMED_ISTREAM,
        },
    },
    core::{HRESULT, Ref, Result, implement},
};

struct Cursor {
    position: u64,
    remote: Option<Remote>,
}

#[implement(IStream)]
struct Stream {
    transfer: Arc<Transfer>,
    index: usize,
    cursor: Mutex<Cursor>,
}

impl Drop for Stream {
    fn drop(&mut self) {
        self.transfer.streams.fetch_sub(1, Ordering::AcqRel);
        let cursor = self
            .cursor
            .get_mut()
            .unwrap_or_else(|error| error.into_inner());
        if let Some(reader) = cursor.remote.take() {
            self.transfer.retire(reader);
        }
    }
}

fn create(transfer: Arc<Transfer>, index: usize, position: u64) -> Result<IStream> {
    if transfer.cancelled() {
        return Err(E_ABORT.into());
    }
    if transfer.streams.fetch_add(1, Ordering::AcqRel) >= 128 {
        transfer.streams.fetch_sub(1, Ordering::AcqRel);
        return Err(STG_E_READFAULT.into());
    }
    Ok(Stream {
        transfer,
        index,
        cursor: Mutex::new(Cursor {
            position,
            remote: None,
        }),
    }
    .into())
}

pub(super) fn medium(transfer: Arc<Transfer>, index: usize) -> Result<STGMEDIUM> {
    let stream = create(transfer, index, 0)?;
    Ok(STGMEDIUM {
        tymed: TYMED_ISTREAM.0 as u32,
        u: STGMEDIUM_0 {
            pstm: ManuallyDrop::new(Some(stream)),
        },
        pUnkForRelease: ManuallyDrop::new(None),
    })
}

#[allow(non_snake_case)]
impl ISequentialStream_Impl for Stream_Impl {
    fn Read(&self, destination: *mut c_void, length: u32, read: *mut u32) -> HRESULT {
        if !read.is_null() {
            unsafe {
                read.write(0);
            }
        }
        if length == 0 {
            return S_OK;
        }
        if destination.is_null() || length as usize > isize::MAX as usize {
            return E_POINTER;
        }
        if self.transfer.cancelled() {
            return E_ABORT;
        }
        let Some(manifest) = self.transfer.manifest.get() else {
            return STG_E_READFAULT;
        };
        let Some(entry) = manifest.entries.get(self.index) else {
            return STG_E_READFAULT;
        };
        // 少数接收端会请求目录对应的空流；目录始终不打开远端文件句柄。
        if entry.directory {
            return S_FALSE;
        }
        let Ok(mut cursor) = self.cursor.try_lock() else {
            return STG_E_LOCKVIOLATION;
        };
        let mut copied = 0u32;
        let result: terminal_core::Result<()> = (|| {
            while copied < length && cursor.position < entry.size {
                if self.transfer.cancelled() {
                    return Err(terminal_core::SessionError::cancelled());
                }
                if cursor.remote.is_none() {
                    let transfer = self.transfer.clone();
                    let index = self.index;
                    cursor.remote = Some(super::wait::wait(&self.transfer, async move {
                        transfer.open(index).await
                    })?);
                }
                let count = u64::from(length - copied)
                    .min(64 * 1024)
                    .min(entry.size - cursor.position) as usize;
                let transfer = self.transfer.clone();
                let remote = cursor.remote.as_ref().unwrap().clone();
                let position = cursor.position;
                let size = entry.size;
                // 后台只持有自有的 64 KiB 缓冲，不把接收端的裸指针跨线程传递。
                let buffer = super::wait::wait(&self.transfer, async move {
                    let mut buffer = vec![0; count];
                    let count = transfer.read(&remote, position, size, &mut buffer).await?;
                    buffer.truncate(count);
                    Ok(buffer)
                })?;
                if buffer.is_empty() {
                    return Err(terminal_files::file_error(
                        "拖出源在读取期间缩短，目标可能保留部分文件",
                    ));
                }
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        buffer.as_ptr(),
                        destination.cast::<u8>().add(copied as usize),
                        buffer.len(),
                    );
                }
                let start = cursor.position;
                cursor.position += buffer.len() as u64;
                copied += buffer.len() as u32;
                self.transfer
                    .delivered(self.index, start..cursor.position)?;
            }
            Ok(())
        })();
        if !read.is_null() {
            unsafe {
                read.write(copied);
            }
        }
        if let Err(error) = result {
            self.transfer.fail(error.message);
            return if self.transfer.cancelled() {
                E_ABORT
            } else {
                STG_E_READFAULT
            };
        }
        if self.transfer.cancelled() {
            return E_ABORT;
        }
        if copied == length { S_OK } else { S_FALSE }
    }

    fn Write(&self, _source: *const c_void, _length: u32, written: *mut u32) -> HRESULT {
        if !written.is_null() {
            unsafe {
                written.write(0);
            }
        }
        STG_E_ACCESSDENIED
    }
}

#[allow(non_snake_case)]
impl IStream_Impl for Stream_Impl {
    fn Seek(&self, delta: i64, origin: STREAM_SEEK, new_position: *mut u64) -> Result<()> {
        if self.transfer.cancelled() {
            return Err(E_ABORT.into());
        }
        let manifest = self.transfer.manifest.get().ok_or(STG_E_READFAULT)?;
        let size = manifest
            .entries
            .get(self.index)
            .ok_or(STG_E_READFAULT)?
            .size;
        let mut cursor = self
            .cursor
            .try_lock()
            .map_err(|_| windows::core::Error::from(STG_E_LOCKVIOLATION))?;
        let base = match origin {
            STREAM_SEEK_SET => 0,
            STREAM_SEEK_CUR => cursor.position,
            STREAM_SEEK_END => size,
            _ => return Err(STG_E_INVALIDFUNCTION.into()),
        };
        cursor.position = u64::try_from(i128::from(base) + i128::from(delta))
            .map_err(|_| windows::core::Error::from(STG_E_INVALIDFUNCTION))?;
        if !new_position.is_null() {
            unsafe {
                new_position.write(cursor.position);
            }
        }
        Ok(())
    }

    fn SetSize(&self, _size: u64) -> Result<()> {
        Err(STG_E_ACCESSDENIED.into())
    }

    fn CopyTo(
        &self,
        destination: Ref<'_, IStream>,
        length: u64,
        read: *mut u64,
        written: *mut u64,
    ) -> Result<()> {
        if !read.is_null() {
            unsafe {
                read.write(0);
            }
        }
        if !written.is_null() {
            unsafe {
                written.write(0);
            }
        }
        let destination = destination.as_ref().ok_or(E_POINTER)?;
        let mut buffer = [0u8; 64 * 1024];
        let (mut total_read, mut total_written) = (0u64, 0u64);
        while total_read < length {
            let mut count = 0u32;
            let result = self.Read(
                buffer.as_mut_ptr().cast(),
                (length - total_read).min(buffer.len() as u64) as u32,
                &mut count,
            );
            total_read += u64::from(count);
            if !read.is_null() {
                unsafe {
                    read.write(total_read);
                }
            }
            result.ok()?;
            if count == 0 {
                break;
            }
            let mut copied = 0u32;
            let result =
                unsafe { destination.Write(buffer.as_ptr().cast(), count, Some(&mut copied)) };
            if copied > count {
                return Err(STG_E_READFAULT.into());
            }
            total_written += u64::from(copied);
            if !written.is_null() {
                unsafe {
                    written.write(total_written);
                }
            }
            result.ok()?;
            if copied != count {
                return Err(STG_E_READFAULT.into());
            }
        }
        Ok(())
    }

    fn Commit(&self, _flags: &STGC) -> Result<()> {
        Ok(())
    }
    fn Revert(&self) -> Result<()> {
        Err(STG_E_INVALIDFUNCTION.into())
    }
    fn LockRegion(&self, _offset: u64, _length: u64, _kind: &LOCKTYPE) -> Result<()> {
        Err(STG_E_INVALIDFUNCTION.into())
    }
    fn UnlockRegion(&self, _offset: u64, _length: u64, _kind: u32) -> Result<()> {
        Err(STG_E_INVALIDFUNCTION.into())
    }

    fn Stat(&self, output: *mut STATSTG, flags: &STATFLAG) -> Result<()> {
        if output.is_null() {
            return Err(E_POINTER.into());
        }
        if self.transfer.cancelled() {
            return Err(E_ABORT.into());
        }
        let manifest = self.transfer.manifest.get().ok_or(STG_E_READFAULT)?;
        let entry = manifest.entries.get(self.index).ok_or(STG_E_READFAULT)?;
        let mut stat = STATSTG {
            r#type: STGTY_STREAM.0 as u32,
            cbSize: entry.size,
            grfMode: STGM_READ,
            ..Default::default()
        };
        if flags.0 & STATFLAG_NONAME.0 == 0 {
            let name: Vec<u16> = entry.relative.encode_utf16().chain([0]).collect();
            let pointer = unsafe { CoTaskMemAlloc(name.len() * size_of::<u16>()) }.cast::<u16>();
            if pointer.is_null() {
                return Err(windows::Win32::Foundation::E_OUTOFMEMORY.into());
            }
            unsafe {
                std::ptr::copy_nonoverlapping(name.as_ptr(), pointer, name.len());
            }
            stat.pwcsName = windows::core::PWSTR(pointer);
        }
        unsafe {
            output.write(stat);
        }
        Ok(())
    }

    fn Clone(&self) -> Result<IStream> {
        create(
            self.transfer.clone(),
            self.index,
            self.cursor
                .try_lock()
                .map_err(|_| windows::core::Error::from(STG_E_LOCKVIOLATION))?
                .position,
        )
    }
}
