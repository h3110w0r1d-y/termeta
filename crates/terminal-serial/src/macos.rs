//! 按IOKit的设备角色选择拨入节点，不通过路径前缀推断端口类型。
use core_foundation::{
    base::{CFType, TCFType, kCFAllocatorDefault},
    string::CFString,
};
use io_kit_sys::{
    IOIteratorNext, IOObjectRelease, IORegistryEntryCreateCFProperty, IOServiceGetMatchingServices,
    IOServiceMatching, kIOMasterPortDefault,
    serial::keys::{kIODialinDeviceKey, kIOSerialBSDServiceValue},
    types::io_object_t,
};
use std::{collections::HashSet, ffi::CStr};
use terminal_core::{Result, SessionError};

struct IoObject(io_object_t);
impl Drop for IoObject {
    fn drop(&mut self) {
        // 迭代器和服务句柄均由IOKit转移所有权，所有返回分支都须释放。
        unsafe { IOObjectRelease(self.0) };
    }
}

fn enumeration_error() -> SessionError {
    SessionError::new(
        "SERIAL_ENUM_FAILED",
        "serial",
        "无法从系统读取串口拨入设备信息，请刷新后重试或手动输入设备路径",
    )
}

pub fn dialin_ports(
    mut ports: Vec<serialport::SerialPortInfo>,
) -> Result<Vec<serialport::SerialPortInfo>> {
    let paths = dialin_paths()?;
    // 保留串口库提供的USB产品等元数据，仅按系统声明的拨入节点精确匹配。
    ports.retain(|port| paths.contains(&port.port_name));
    Ok(ports)
}

fn dialin_paths() -> Result<HashSet<String>> {
    // 系统常量是有效的NUL结尾字符串，只在此处转换属性键。
    let key = unsafe { CStr::from_ptr(kIODialinDeviceKey) }
        .to_str()
        .map_err(|_| enumeration_error())?;
    let key = CFString::new(key);
    let mut iterator = 0;
    // GetMatchingServices消费matching字典，不再由本地CF包装器重复释放。
    let matching = unsafe { IOServiceMatching(kIOSerialBSDServiceValue) };
    if matching.is_null() {
        return Err(enumeration_error());
    }
    let status =
        unsafe { IOServiceGetMatchingServices(kIOMasterPortDefault, matching, &mut iterator) };
    if status != 0 || iterator == 0 {
        return Err(enumeration_error());
    }
    let iterator = IoObject(iterator);
    let mut paths = HashSet::new();
    loop {
        // 零句柄表示枚举结束，每个非零服务句柄交给RAII守卫。
        let service = unsafe { IOIteratorNext(iterator.0) };
        if service == 0 {
            break;
        }
        let service = IoObject(service);
        let value = unsafe {
            IORegistryEntryCreateCFProperty(
                service.0,
                key.as_concrete_TypeRef(),
                kCFAllocatorDefault,
                0,
            )
        };
        // 没有拨入节点的服务不进入候选项，不回退为callout或猜测路径。
        if value.is_null() {
            continue;
        }
        // Create规则返回的属性由CF包装器释放，先检查类型再读取字符串。
        let value = unsafe { CFType::wrap_under_create_rule(value) };
        let path = value
            .downcast::<CFString>()
            .ok_or_else(enumeration_error)?
            .to_string();
        if !path.is_empty() {
            paths.insert(path);
        }
    }
    // 设备变动导致迭代失效时不能把不完整结果当作成功。
    if unsafe { io_kit_sys::IOIteratorIsValid(iterator.0) } == 0 {
        return Err(enumeration_error());
    }
    Ok(paths)
}
