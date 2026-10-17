//! 备份专用内存 SQLite 边界；不接触活动 SQLCipher 连接或磁盘临时文件。
use super::backup::{Cell, MAX_DATABASE_BYTES, MAX_ROWS, TABLES};
use crate::{Result, StorageError};
use libsqlite3_sys as ffi;
use std::{
    ffi::{CStr, CString, c_char, c_int, c_void},
    ptr,
};
use zeroize::{Zeroize, Zeroizing};

fn invalid() -> StorageError {
    StorageError::Validation("备份数据库损坏、结构不兼容或超过资源限额")
}

struct MemoryDb(*mut ffi::sqlite3);
impl Drop for MemoryDb {
    fn drop(&mut self) {
        // 所有语句均先于连接释放，连接不跨线程共享。
        unsafe {
            ffi::sqlite3_close(self.0);
        }
    }
}
struct Statement(*mut ffi::sqlite3_stmt);
impl Drop for Statement {
    fn drop(&mut self) {
        unsafe {
            ffi::sqlite3_finalize(self.0);
        }
    }
}
impl MemoryDb {
    fn new() -> Result<Self> {
        let mut raw = ptr::null_mut();
        // 固定内存名称禁止由备份内容指定文件或 URI。
        let code = unsafe {
            ffi::sqlite3_open_v2(
                c":memory:".as_ptr(),
                &mut raw,
                ffi::SQLITE_OPEN_READWRITE | ffi::SQLITE_OPEN_CREATE | ffi::SQLITE_OPEN_NOMUTEX,
                ptr::null(),
            )
        };
        let db = Self(raw);
        if code != ffi::SQLITE_OK {
            return Err(invalid());
        }
        unsafe {
            ffi::sqlite3_limit(raw, ffi::SQLITE_LIMIT_LENGTH, MAX_DATABASE_BYTES as c_int);
            ffi::sqlite3_limit(raw, ffi::SQLITE_LIMIT_SQL_LENGTH, 32 * 1024);
            ffi::sqlite3_limit(raw, ffi::SQLITE_LIMIT_COLUMN, 32);
            ffi::sqlite3_limit(raw, ffi::SQLITE_LIMIT_EXPR_DEPTH, 32);
            ffi::sqlite3_db_config(
                raw,
                ffi::SQLITE_DBCONFIG_TRUSTED_SCHEMA,
                0,
                ptr::null_mut::<c_int>(),
            );
            ffi::sqlite3_db_config(
                raw,
                ffi::SQLITE_DBCONFIG_DEFENSIVE,
                1,
                ptr::null_mut::<c_int>(),
            );
            ffi::sqlite3_db_config(
                raw,
                ffi::SQLITE_DBCONFIG_ENABLE_TRIGGER,
                0,
                ptr::null_mut::<c_int>(),
            );
            ffi::sqlite3_db_config(
                raw,
                ffi::SQLITE_DBCONFIG_ENABLE_VIEW,
                0,
                ptr::null_mut::<c_int>(),
            );
            ffi::sqlite3_db_config(
                raw,
                ffi::SQLITE_DBCONFIG_ENABLE_LOAD_EXTENSION,
                0,
                ptr::null_mut::<c_int>(),
            );
        }
        db.execute("PRAGMA temp_store=MEMORY")?;
        db.execute("PRAGMA secure_delete=ON")?;
        db.execute("PRAGMA cipher_memory_security=ON")?;
        Ok(db)
    }
    fn prepare(&self, sql: &str) -> Result<Statement> {
        let text = CString::new(sql).map_err(|_| invalid())?;
        let mut stmt = ptr::null_mut();
        let code = unsafe {
            ffi::sqlite3_prepare_v2(self.0, text.as_ptr(), -1, &mut stmt, ptr::null_mut())
        };
        let statement = Statement(stmt);
        if code != ffi::SQLITE_OK || stmt.is_null() {
            return Err(invalid());
        }
        Ok(statement)
    }
    fn execute(&self, sql: &str) -> Result<()> {
        let stmt = self.prepare(sql)?;
        loop {
            match unsafe { ffi::sqlite3_step(stmt.0) } {
                ffi::SQLITE_ROW => continue,
                ffi::SQLITE_DONE => return Ok(()),
                _ => return Err(invalid()),
            }
        }
    }
}

fn bind(stmt: &Statement, values: &[Cell]) -> Result<()> {
    for (i, value) in values.iter().enumerate() {
        let i = i as c_int + 1;
        let code = unsafe {
            match value {
                Cell::Null => ffi::sqlite3_bind_null(stmt.0, i),
                Cell::Integer(value) => ffi::sqlite3_bind_int64(stmt.0, i, *value),
                Cell::Text(value) => ffi::sqlite3_bind_text(
                    stmt.0,
                    i,
                    value.as_ptr().cast(),
                    value.len() as c_int,
                    ffi::SQLITE_TRANSIENT(),
                ),
                Cell::Blob(value) => ffi::sqlite3_bind_blob(
                    stmt.0,
                    i,
                    value.as_ptr().cast(),
                    value.len() as c_int,
                    ffi::SQLITE_TRANSIENT(),
                ),
            }
        };
        if code != ffi::SQLITE_OK {
            return Err(invalid());
        }
    }
    Ok(())
}
fn cells(stmt: &Statement, count: usize) -> Result<Vec<Cell>> {
    let mut values = Vec::with_capacity(count);
    for i in 0..count as c_int {
        let kind = unsafe { ffi::sqlite3_column_type(stmt.0, i) };
        let value = match kind {
            ffi::SQLITE_NULL => Cell::Null,
            ffi::SQLITE_INTEGER => Cell::Integer(unsafe { ffi::sqlite3_column_int64(stmt.0, i) }),
            ffi::SQLITE_TEXT | ffi::SQLITE_BLOB => {
                let len = unsafe { ffi::sqlite3_column_bytes(stmt.0, i) };
                if len < 0 || len as usize > MAX_DATABASE_BYTES {
                    return Err(invalid());
                }
                let data = unsafe { ffi::sqlite3_column_blob(stmt.0, i) };
                let bytes = if len == 0 {
                    &[][..]
                } else {
                    if data.is_null() {
                        return Err(invalid());
                    }
                    // SQLite 缓冲区仅在当前 step 有效，此处立即复制到受控缓冲区。
                    unsafe { std::slice::from_raw_parts(data.cast::<u8>(), len as usize) }
                };
                if kind == ffi::SQLITE_TEXT {
                    Cell::Text(Zeroizing::new(
                        std::str::from_utf8(bytes)
                            .map_err(|_| invalid())?
                            .to_owned(),
                    ))
                } else {
                    Cell::Blob(Zeroizing::new(bytes.to_vec()))
                }
            }
            _ => return Err(invalid()),
        };
        values.push(value);
    }
    Ok(values)
}

pub(super) fn encode(data: &[Vec<Vec<Cell>>]) -> Result<Zeroizing<Vec<u8>>> {
    let db = MemoryDb::new()?;
    db.execute("PRAGMA page_size=4096")?;
    db.execute("PRAGMA max_page_count=32768")?;
    db.execute("PRAGMA user_version=2")?;
    db.execute("BEGIN")?;
    for (table, rows) in TABLES.iter().zip(data) {
        db.execute(&table.schema())?;
        let stmt = db.prepare(&table.insert())?;
        for row in rows {
            bind(&stmt, row)?;
            if unsafe { ffi::sqlite3_step(stmt.0) } != ffi::SQLITE_DONE {
                return Err(invalid());
            }
            unsafe {
                ffi::sqlite3_reset(stmt.0);
                ffi::sqlite3_clear_bindings(stmt.0);
            }
        }
    }
    db.execute("COMMIT")?;
    let mut size = 0i64;
    let data = unsafe { ffi::sqlite3_serialize(db.0, c"main".as_ptr(), &mut size, 0) };
    if data.is_null() {
        return Err(invalid());
    }
    // serialize 由 SQLite 分配；无论校验成功与否，释放前先擦除明文副本。
    let result = if size <= 0 || size as usize > MAX_DATABASE_BYTES {
        Err(invalid())
    } else {
        Ok(Zeroizing::new(
            unsafe { std::slice::from_raw_parts(data, size as usize) }.to_vec(),
        ))
    };
    if size > 0 {
        unsafe { std::slice::from_raw_parts_mut(data, size as usize) }.zeroize();
    }
    unsafe {
        ffi::sqlite3_free(data.cast());
    }
    result
}

unsafe extern "C" fn read_authorizer(
    _: *mut c_void,
    action: c_int,
    table: *const c_char,
    _: *const c_char,
    _: *const c_char,
    _: *const c_char,
) -> c_int {
    // 即使输入试图伪装成视图或虚拟表，也没有函数调用、写入、ATTACH 或 PRAGMA 权限。
    if action == ffi::SQLITE_SELECT {
        return ffi::SQLITE_OK;
    }
    if action == ffi::SQLITE_READ && !table.is_null() {
        // SQLite 在回调期间保证名称指针为有效的 NUL 结尾字符串。
        let name = unsafe { CStr::from_ptr(table) }.to_bytes();
        if matches!(name, b"sqlite_master" | b"sqlite_schema")
            || TABLES.iter().any(|table| table.name.as_bytes() == name)
        {
            return ffi::SQLITE_OK;
        }
    }
    ffi::SQLITE_DENY
}
unsafe extern "C" fn progress(data: *mut c_void) -> c_int {
    let budget = unsafe { &mut *data.cast::<u64>() };
    *budget = budget.saturating_sub(1);
    c_int::from(*budget == 0)
}

pub(super) fn decode(bytes: Zeroizing<Vec<u8>>) -> Result<Vec<Vec<Vec<Cell>>>> {
    if bytes.len() < 100
        || bytes.len() > MAX_DATABASE_BYTES
        || !bytes.starts_with(b"SQLite format 3\0")
        || bytes[18] != 1
        || bytes[19] != 1
    {
        return Err(invalid());
    }
    let db = MemoryDb::new()?;
    // 不让 SQLite 取得 Vec 的所有权；READONLY 禁止扩容/改写，bytes 保持存活直至连接先释放。
    let mut budget = 20_000u64;
    let result = (|| {
        let code = unsafe {
            ffi::sqlite3_deserialize(
                db.0,
                c"main".as_ptr(),
                bytes.as_ptr().cast_mut(),
                bytes.len() as i64,
                bytes.len() as i64,
                ffi::SQLITE_DESERIALIZE_READONLY,
            )
        };
        if code != ffi::SQLITE_OK {
            return Err(invalid());
        }
        db.execute("PRAGMA query_only=ON")?;
        let version = db.prepare("PRAGMA user_version")?;
        if unsafe { ffi::sqlite3_step(version.0) } != ffi::SQLITE_ROW {
            return Err(invalid());
        }
        let version_number = unsafe { ffi::sqlite3_column_int64(version.0, 0) };
        // 旧版九表快照没有片段，按空集合导入，仍严格校验原有所有表。
        let expected_tables = match version_number {
            1 => &TABLES[..9],
            2 => TABLES,
            _ => return Err(invalid()),
        };
        drop(version);
        unsafe {
            ffi::sqlite3_set_authorizer(db.0, Some(read_authorizer), ptr::null_mut());
        }
        unsafe {
            ffi::sqlite3_progress_handler(
                db.0,
                1000,
                Some(progress),
                (&mut budget as *mut u64).cast(),
            );
        }
        let schema = db.prepare("SELECT name,type,sql FROM sqlite_schema ORDER BY name")?;
        let mut seen = std::collections::HashSet::new();
        loop {
            match unsafe { ffi::sqlite3_step(schema.0) } {
                ffi::SQLITE_DONE => break,
                ffi::SQLITE_ROW => {
                    let row = cells(&schema, 3)?;
                    let table = expected_tables
                        .iter()
                        .find(|table| row[0].text().ok() == Some(table.name))
                        .ok_or_else(invalid)?;
                    if row[1].text()? != "table" || row[2].text()? != table.schema() {
                        return Err(invalid());
                    }
                    if !seen.insert(table.name) {
                        return Err(invalid());
                    }
                }
                _ => return Err(invalid()),
            }
        }
        if seen.len() != expected_tables.len() {
            return Err(invalid());
        }
        drop(schema);
        let mut total = 0usize;
        let mut total_bytes = 0usize;
        let mut output = Vec::new();
        for table in expected_tables {
            let stmt = db.prepare(&table.select())?;
            let mut rows = Vec::new();
            loop {
                match unsafe { ffi::sqlite3_step(stmt.0) } {
                    ffi::SQLITE_DONE => break,
                    ffi::SQLITE_ROW => {
                        total += 1;
                        if total > MAX_ROWS {
                            return Err(invalid());
                        }
                        let row = cells(&stmt, table.columns().len())?;
                        total_bytes = total_bytes
                            .checked_add(row.iter().map(Cell::size).sum::<usize>())
                            .ok_or_else(invalid)?;
                        if total_bytes > MAX_DATABASE_BYTES {
                            return Err(invalid());
                        }
                        rows.push(row);
                    }
                    _ => return Err(invalid()),
                }
            }
            output.push(rows);
        }
        output.resize_with(TABLES.len(), Vec::new);
        // 进度回调指针在 budget 离开作用域前撤销。
        unsafe {
            ffi::sqlite3_progress_handler(db.0, 0, None, ptr::null_mut());
        }
        Ok(output)
    })();
    // 必须先关闭 SQLite 再擦除 bytes，避免解引用已释放的反序列化缓冲区。
    drop(db);
    drop(bytes);
    result
}
