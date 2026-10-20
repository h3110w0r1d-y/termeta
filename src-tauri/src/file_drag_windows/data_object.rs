//! Shell 虚拟文件协议：目录描述符与内容索引共用一份不可变清单。
use super::{manifest::Manifest, stream, transfer::Transfer};
use std::{
    mem::ManuallyDrop,
    sync::{Arc, atomic::Ordering},
};
use windows::{
    Win32::{
        Foundation::{
            DV_E_DVASPECT, DV_E_DVTARGETDEVICE, DV_E_FORMATETC, DV_E_LINDEX, DV_E_TYMED, E_ABORT,
            E_FAIL, E_NOTIMPL, E_OUTOFMEMORY, E_POINTER, FILETIME, GlobalFree, HGLOBAL,
            OLE_E_ADVISENOTSUPPORTED, S_OK, STG_E_READFAULT,
        },
        Storage::FileSystem::{FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_NORMAL},
        System::{
            Com::{
                DATADIR_GET, DVASPECT_CONTENT, FORMATETC, IAdviseSink, IBindCtx, IDataObject,
                IDataObject_Impl, IEnumFORMATETC, IEnumSTATDATA, STGMEDIUM, STGMEDIUM_0,
                TYMED_HGLOBAL, TYMED_ISTREAM,
            },
            DataExchange::RegisterClipboardFormatW,
            Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock},
            Ole::{DROPEFFECT_COPY, ReleaseStgMedium},
        },
        UI::Shell::{
            FD_ATTRIBUTES, FD_FILESIZE, FD_PROGRESSUI, FD_UNICODE, FD_WRITESTIME, FILEDESCRIPTORW,
            FILEGROUPDESCRIPTORW, IDataObjectAsyncCapability, IDataObjectAsyncCapability_Impl,
            SHCreateStdEnumFmtEtc,
        },
    },
    core::{BOOL, HRESULT, Ref, Result, implement, w},
};

#[derive(Clone, Copy)]
pub(super) struct Formats {
    descriptor: u16,
    contents: u16,
    preferred: u16,
    performed: u16,
    succeeded: u16,
}
impl Formats {
    pub fn register() -> Result<Self> {
        fn register(name: windows::core::PCWSTR) -> Result<u16> {
            let value = unsafe { RegisterClipboardFormatW(name) };
            if value == 0 {
                Err(windows::core::Error::from_thread())
            } else {
                Ok(value as u16)
            }
        }
        Ok(Self {
            descriptor: register(w!("FileGroupDescriptorW"))?,
            contents: register(w!("FileContents"))?,
            preferred: register(w!("Preferred DropEffect"))?,
            performed: register(w!("Performed DropEffect"))?,
            succeeded: register(w!("Paste Succeeded"))?,
        })
    }
    fn format(&self, id: u16, tymed: u32, index: i32) -> FORMATETC {
        FORMATETC {
            cfFormat: id,
            ptd: std::ptr::null_mut(),
            dwAspect: DVASPECT_CONTENT.0,
            lindex: index,
            tymed,
        }
    }
}

struct GlobalMemory(HGLOBAL);
impl Drop for GlobalMemory {
    fn drop(&mut self) {
        unsafe {
            let _ = GlobalFree(Some(self.0));
        }
    }
}

fn global_medium(size: usize, write: impl FnOnce(*mut u8)) -> Result<STGMEDIUM> {
    let memory = GlobalMemory(unsafe { GlobalAlloc(GMEM_MOVEABLE, size)? });
    let pointer = unsafe { GlobalLock(memory.0) }.cast::<u8>();
    if pointer.is_null() {
        return Err(E_OUTOFMEMORY.into());
    }
    write(pointer);
    unsafe {
        let _ = GlobalUnlock(memory.0);
    }
    let handle = memory.0;
    std::mem::forget(memory);
    Ok(STGMEDIUM {
        tymed: TYMED_HGLOBAL.0 as u32,
        u: STGMEDIUM_0 { hGlobal: handle },
        pUnkForRelease: ManuallyDrop::new(None),
    })
}

fn descriptors(manifest: &Manifest) -> Result<STGMEDIUM> {
    let header = std::mem::offset_of!(FILEGROUPDESCRIPTORW, fgd);
    let size = header + manifest.entries.len() * size_of::<FILEDESCRIPTORW>();
    global_medium(size, |pointer| unsafe {
        pointer
            .cast::<u32>()
            .write_unaligned(manifest.entries.len() as u32);
        for (index, entry) in manifest.entries.iter().enumerate() {
            let mut descriptor = FILEDESCRIPTORW::default();
            descriptor.dwFlags = (FD_ATTRIBUTES.0 | FD_UNICODE.0 | FD_PROGRESSUI.0) as u32;
            descriptor.dwFileAttributes = if entry.directory {
                FILE_ATTRIBUTE_DIRECTORY.0
            } else {
                FILE_ATTRIBUTE_NORMAL.0
            };
            if !entry.directory {
                descriptor.dwFlags |= FD_FILESIZE.0 as u32;
                descriptor.nFileSizeHigh = (entry.size >> 32) as u32;
                descriptor.nFileSizeLow = entry.size as u32;
            }
            if let Some(modified) = entry.modified {
                let ticks = (u64::from(modified) + 11644473600) * 10000000;
                descriptor.dwFlags |= FD_WRITESTIME.0 as u32;
                descriptor.ftLastWriteTime = FILETIME {
                    dwLowDateTime: ticks as u32,
                    dwHighDateTime: (ticks >> 32) as u32,
                };
            }
            // packed 描述符字段不能被借用；先填充对齐的数组，再按值赋回。
            // 清单已验证 UTF-16 长度和每个路径分量，终止符由零初始化保留。
            let mut file_name = [0u16; 260];
            for (target, value) in file_name.iter_mut().zip(entry.relative.encode_utf16()) {
                *target = value;
            }
            descriptor.cFileName = file_name;
            pointer
                .add(header + index * size_of::<FILEDESCRIPTORW>())
                .cast::<FILEDESCRIPTORW>()
                .write_unaligned(descriptor);
        }
    })
}

#[implement(IDataObject, IDataObjectAsyncCapability)]
pub(super) struct DataObject {
    pub transfer: Arc<Transfer>,
    pub formats: Formats,
}

impl DataObject {
    fn validate(&self, format: *const FORMATETC) -> Result<FORMATETC> {
        // 复制本次 COM 调用提供的格式描述，不保存调用者的结构指针。
        let format = unsafe { format.as_ref() }.copied().ok_or(E_POINTER)?;
        if !format.ptd.is_null() {
            return Err(DV_E_DVTARGETDEVICE.into());
        }
        if format.dwAspect != DVASPECT_CONTENT.0 {
            return Err(DV_E_DVASPECT.into());
        }
        Ok(format)
    }
    fn requested(&self, format: &FORMATETC, query: bool) -> Result<()> {
        let expected = if format.cfFormat == self.formats.contents {
            if format.lindex < 0 && !(query && format.lindex == -1) {
                return Err(DV_E_LINDEX.into());
            }
            let length = self
                .transfer
                .manifest
                .get()
                .map_or(super::manifest::MAX_ENTRIES, |manifest| {
                    manifest.entries.len()
                });
            if format.lindex >= length as i32 {
                return Err(DV_E_LINDEX.into());
            }
            TYMED_ISTREAM.0 as u32
        } else if format.cfFormat == self.formats.descriptor
            || format.cfFormat == self.formats.preferred
        {
            if format.lindex != -1 {
                return Err(DV_E_LINDEX.into());
            }
            TYMED_HGLOBAL.0 as u32
        } else {
            return Err(DV_E_FORMATETC.into());
        };
        if format.tymed & expected == 0 {
            return Err(DV_E_TYMED.into());
        }
        if self.transfer.cancelled() {
            return Err(E_ABORT.into());
        }
        Ok(())
    }
}

#[allow(non_snake_case)]
impl IDataObject_Impl for DataObject_Impl {
    fn GetData(&self, format: *const FORMATETC) -> Result<STGMEDIUM> {
        let format = self.validate(format)?;
        self.requested(&format, false)?;
        if format.cfFormat == self.formats.preferred {
            return global_medium(size_of::<u32>(), |pointer| unsafe {
                pointer.cast::<u32>().write_unaligned(DROPEFFECT_COPY.0);
            });
        }
        // 只在接收端请求描述符/内容时扫描；QueryGetData 和开始手势不等待网络。
        let transfer = self.transfer.clone();
        let manifest =
            super::wait::wait(&self.transfer, async move { transfer.get_manifest().await })
                .map_err(|error| {
                    self.transfer.fail(error.message);
                    windows::core::Error::from(if self.transfer.cancelled() {
                        E_ABORT
                    } else {
                        STG_E_READFAULT
                    })
                })?;
        if format.cfFormat == self.formats.descriptor {
            let medium = descriptors(&manifest)?;
            self.transfer
                .descriptors_served
                .store(true, Ordering::Release);
            return Ok(medium);
        }
        if format.lindex as usize >= manifest.entries.len() {
            return Err(DV_E_LINDEX.into());
        }
        stream::medium(self.transfer.clone(), format.lindex as usize)
    }

    fn GetDataHere(&self, _format: *const FORMATETC, _medium: *mut STGMEDIUM) -> Result<()> {
        Err(E_NOTIMPL.into())
    }
    fn QueryGetData(&self, format: *const FORMATETC) -> HRESULT {
        self.validate(format)
            .and_then(|format| self.requested(&format, true))
            .map(|_| S_OK)
            .unwrap_or_else(|error| error.code())
    }
    fn GetCanonicalFormatEtc(&self, _input: *const FORMATETC, output: *mut FORMATETC) -> HRESULT {
        if output.is_null() {
            return E_POINTER;
        }
        unsafe {
            (*output).ptd = std::ptr::null_mut();
        }
        E_NOTIMPL
    }

    fn SetData(
        &self,
        format: *const FORMATETC,
        medium: *const STGMEDIUM,
        release: BOOL,
    ) -> Result<()> {
        let format = self.validate(format)?;
        if format.cfFormat != self.formats.performed && format.cfFormat != self.formats.succeeded {
            return Err(DV_E_FORMATETC.into());
        }
        if format.lindex != -1 {
            return Err(DV_E_LINDEX.into());
        }
        let medium = unsafe { medium.as_ref() }.ok_or(E_POINTER)?;
        if format.tymed & TYMED_HGLOBAL.0 as u32 == 0 || medium.tymed != TYMED_HGLOBAL.0 as u32 {
            return Err(DV_E_TYMED.into());
        }
        let handle = unsafe { medium.u.hGlobal };
        if unsafe { GlobalSize(handle) } < size_of::<u32>() {
            return Err(DV_E_FORMATETC.into());
        }
        let pointer = unsafe { GlobalLock(handle) }.cast::<u32>();
        if pointer.is_null() {
            return Err(E_FAIL.into());
        }
        let effect = unsafe { pointer.read_unaligned() };
        unsafe {
            let _ = GlobalUnlock(handle);
        }
        {
            let mut operation = self.transfer.operation.lock().unwrap();
            if operation.sealed || self.transfer.cancelled() {
                return Err(E_ABORT.into());
            }
            // 仅记录接收端反馈；即使报告 Move 也绝不删除远端源。
            if format.cfFormat == self.formats.performed {
                operation.performed = Some(effect);
            }
        }
        if release.as_bool() {
            // 成功接受所有权后仅释放一次；失败路径仍由调用者释放。
            let mut owned = unsafe { std::ptr::read(medium) };
            unsafe {
                ReleaseStgMedium(&mut owned);
            }
        }
        Ok(())
    }

    fn EnumFormatEtc(&self, direction: u32) -> Result<IEnumFORMATETC> {
        if direction != DATADIR_GET.0 as u32 {
            return Err(E_NOTIMPL.into());
        }
        unsafe {
            SHCreateStdEnumFmtEtc(&[
                self.formats
                    .format(self.formats.descriptor, TYMED_HGLOBAL.0 as u32, -1),
                self.formats
                    .format(self.formats.contents, TYMED_ISTREAM.0 as u32, -1),
                self.formats
                    .format(self.formats.preferred, TYMED_HGLOBAL.0 as u32, -1),
            ])
        }
    }
    fn DAdvise(
        &self,
        _format: *const FORMATETC,
        _flags: u32,
        _sink: Ref<'_, IAdviseSink>,
    ) -> Result<u32> {
        Err(OLE_E_ADVISENOTSUPPORTED.into())
    }
    fn DUnadvise(&self, _connection: u32) -> Result<()> {
        Err(OLE_E_ADVISENOTSUPPORTED.into())
    }
    fn EnumDAdvise(&self) -> Result<IEnumSTATDATA> {
        Err(OLE_E_ADVISENOTSUPPORTED.into())
    }
}

#[allow(non_snake_case)]
impl IDataObjectAsyncCapability_Impl for DataObject_Impl {
    fn SetAsyncMode(&self, enabled: BOOL) -> Result<()> {
        let mut operation = self.transfer.operation.lock().unwrap();
        if operation.sealed || operation.async_started {
            return Err(E_FAIL.into());
        }
        operation.async_enabled = enabled.as_bool();
        Ok(())
    }
    fn GetAsyncMode(&self) -> Result<BOOL> {
        Ok(self.transfer.operation.lock().unwrap().async_enabled.into())
    }
    fn StartOperation(&self, _reserved: Ref<'_, IBindCtx>) -> Result<()> {
        {
            let mut operation = self.transfer.operation.lock().unwrap();
            if self.transfer.cancelled() || operation.sealed {
                return Err(E_ABORT.into());
            }
            operation.start()?;
        }
        self.transfer.accept();
        Ok(())
    }
    fn InOperation(&self) -> Result<BOOL> {
        let operation = self.transfer.operation.lock().unwrap();
        Ok((operation.async_started && operation.end.is_none() && !operation.sealed).into())
    }
    fn EndOperation(
        &self,
        result: HRESULT,
        _reserved: Ref<'_, IBindCtx>,
        effect: u32,
    ) -> Result<()> {
        let mut operation = self.transfer.operation.lock().unwrap();
        operation.end(result, effect)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_drag_windows::manifest::Entry;
    use windows::Win32::System::Ole::ReleaseStgMedium;

    #[test]
    fn directory_and_empty_directory_have_real_directory_descriptors() {
        let manifest = Manifest {
            entries: vec![
                Entry {
                    relative: "root".into(),
                    remote: "/root".into(),
                    directory: true,
                    size: 0,
                    modified: None,
                },
                Entry {
                    relative: "root\\empty".into(),
                    remote: "/root/empty".into(),
                    directory: true,
                    size: 0,
                    modified: None,
                },
                Entry {
                    relative: "root\\large.bin".into(),
                    remote: "/root/large.bin".into(),
                    directory: false,
                    size: (1u64 << 32) + 17,
                    modified: None,
                },
            ],
        };
        let mut medium = descriptors(&manifest).unwrap();
        unsafe {
            let pointer = GlobalLock(medium.u.hGlobal).cast::<u8>();
            assert!(!pointer.is_null());
            assert_eq!(pointer.cast::<u32>().read_unaligned(), 3);
            let descriptor = |index| {
                pointer
                    .add(
                        std::mem::offset_of!(FILEGROUPDESCRIPTORW, fgd)
                            + index * size_of::<FILEDESCRIPTORW>(),
                    )
                    .cast::<FILEDESCRIPTORW>()
                    .read_unaligned()
            };
            // 断言宏和切片也会借用字段，先按值复制 packed 字段。
            let root_attributes = descriptor(0).dwFileAttributes;
            assert_eq!(root_attributes, FILE_ATTRIBUTE_DIRECTORY.0);
            let empty = descriptor(1);
            let empty_attributes = empty.dwFileAttributes;
            let empty_name = empty.cFileName;
            assert_eq!(empty_attributes, FILE_ATTRIBUTE_DIRECTORY.0);
            assert_eq!(empty.dwFlags & FD_FILESIZE.0 as u32, 0);
            assert_eq!(
                String::from_utf16(&empty_name[..10]).unwrap(),
                "root\\empty"
            );
            assert_eq!(empty_name[10], 0);
            let file = descriptor(2);
            let size_high = file.nFileSizeHigh;
            let size_low = file.nFileSizeLow;
            assert_eq!(size_high, 1);
            assert_eq!(size_low, 17);
            assert_ne!(file.dwFlags & FD_FILESIZE.0 as u32, 0);
            let _ = GlobalUnlock(medium.u.hGlobal);
            ReleaseStgMedium(&mut medium);
        }
    }
}
