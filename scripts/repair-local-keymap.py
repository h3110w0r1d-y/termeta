#!/usr/bin/env python3
# /// script
# requires-python = ">=3.10"
# dependencies = ["argon2-cffi==25.1.0", "cryptography==50.0.2"]
# ///
"""独立修复 macOS 本地加密库中的快捷键命名；默认只预览，不启动 Termeta。"""

import argparse
import copy
from datetime import datetime, timezone
import getpass
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time
import unicodedata
import uuid

CHECK = b"termeta/encrypted-vault/v2"
KDF = "argon2id-v19-m65536-t3-p1"
ACTIONS = (
    "newTab",
    "newConnection",
    "closeTab",
    "nextTab",
    "previousTab",
    "newLocalTerminal",
)
LEGACY = {f"workspace.{action}": f"workbench.{action}" for action in ACTIONS}
COMMANDS = set(LEGACY.values()) | {
    "settings.open",
    "terminal.copy",
    "terminal.paste",
    "terminal.find",
    "terminal.scrollToBottom",
    "terminal.openLink",
}


class RepairError(Exception):
    """只向终端输出预先定义的诊断，避免泄露数据库内容或解密材料。"""


def require(condition, message):
    if not condition:
        raise RepairError(message)


def parse_json(raw):
    def object_pairs(pairs):
        result = {}
        for key, value in pairs:
            require(key not in result, "JSON 包含重复字段，停止修复。")
            result[key] = value
        return result

    def reject_constant(_value):
        raise RepairError("JSON 包含非标准数值，停止修复。")

    return json.loads(
        raw, object_pairs_hook=object_pairs, parse_constant=reject_constant
    )


def check_closed(files):
    # 只检查相关文件，不依据窗口显隐判断应用是否真正退出。
    result = subprocess.run(
        ["/usr/sbin/lsof", "-t", "--", *(str(path) for path in files if path.exists())],
        capture_output=True,
        timeout=15,
        check=False,
    )
    require(
        not result.stdout.strip(), "数据库仍被进程占用，请完全退出 Termeta 后重试。"
    )
    require(
        result.returncode == 1 and not result.stderr.strip(),
        "无法确认数据库已关闭，停止修复。请检查 lsof 权限。",
    )


def fingerprints(files):
    result = {}
    for path in files:
        if path.exists():
            require(
                path.is_file() and not path.is_symlink(),
                "数据库配套路径必须为普通文件。",
            )
            with path.open("rb") as stream:
                digest = hashlib.sha256()
                for block in iter(lambda: stream.read(1024 * 1024), b""):
                    digest.update(block)
            result[path.name] = digest.hexdigest()
    return result


def copy_files(files, directory):
    for source in files:
        if source.exists():
            target = directory / source.name
            with source.open("rb") as src, target.open("xb") as dest:
                os.chmod(target, 0o600)
                shutil.copyfileobj(src, dest)
                dest.flush()
                os.fsync(dest.fileno())
    descriptor = os.open(directory, os.O_RDONLY)
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def unlock_key(metadata):
    try:
        from argon2.low_level import Type, hash_secret_raw
        from cryptography.hazmat.primitives.ciphers.aead import AESGCM
    except ImportError:
        raise RepairError(
            "缺少依赖，请按配套说明使用 uv run 或安装 argon2-cffi、cryptography。"
        ) from None
    header = parse_json(metadata.read_bytes())
    require(
        isinstance(header, dict)
        and header.get("format") == 2
        and header.get("kdf") == KDF
        and header.get("initialized") is True,
        "保险库元数据不是已初始化的加密格式 2，停止修复。",
    )
    require(isinstance(header.get("id"), str), "保险库元数据 ID 类型无效。")
    try:
        uuid.UUID(header["id"])
        salt, nonce, verifier = (
            bytes(header[name]) for name in ("salt", "nonce", "verifier")
        )
        require(
            len(salt) == 16 and len(nonce) == 12 and len(verifier) == len(CHECK) + 16,
            "保险库元数据长度无效。",
        )
    except (KeyError, ValueError, TypeError):
        raise RepairError("保险库元数据字段无效。") from None
    password = getpass.getpass("请输入当前保险库密码（输入不显示）：")
    require(
        bool(password) and len(password.encode("utf-8")) <= 1024, "保险库密码长度无效。"
    )
    key = hash_secret_raw(
        password.encode("utf-8"), salt, 3, 65536, 1, 32, Type.ID, version=19
    )
    del password
    aad = CHECK + header["id"].encode() + KDF.encode() + salt + b"\x01"
    try:
        require(AESGCM(key).decrypt(nonce, verifier, aad) == CHECK, "保险库校验失败。")
    except Exception:
        raise RepairError("密码不正确或保险库元数据校验失败，未修改数据库。") from None
    return key


def sql_query(executable, database, key, statements, readonly=True):
    # 密钥只通过子进程标准输入传递，不放入命令行、环境变量或临时 SQL 文件。
    token = uuid.uuid4().hex
    sql = [
        ".bail on",
        ".timeout 3000",
        ".headers off",
        ".mode list",
        f"PRAGMA key = \"x'{key.hex()}'\";",
        "PRAGMA cipher_compatibility = 4;",
        "PRAGMA cipher_memory_security = ON;",
        "PRAGMA temp_store = MEMORY;",
        "PRAGMA foreign_keys = ON;",
    ]
    for index, statement in enumerate(statements):
        sql += [
            f"SELECT '{token}_{index}_BEGIN';",
            statement,
            f"SELECT '{token}_{index}_END';",
        ]
    result = subprocess.run(
        [
            executable,
            "-init",
            "/dev/null",
            "-batch",
            "-bail",
            *(["-readonly"] if readonly else []),
            str(database),
        ],
        input="\n".join(sql) + "\n",
        text=True,
        capture_output=True,
        timeout=120,
        check=False,
    )
    require(
        result.returncode == 0,
        "SQLCipher 操作失败（可能为密钥、锁、约束或完整性问题）；未输出原始 SQL/数据库内容。",
    )
    lines = result.stdout.splitlines()
    outputs = []
    for index in range(len(statements)):
        try:
            start, finish = lines.index(f"{token}_{index}_BEGIN"), lines.index(
                f"{token}_{index}_END"
            )
        except ValueError:
            raise RepairError("SQLCipher 返回格式无效，停止修复。") from None
        outputs.append(lines[start + 1 : finish])
    return outputs


def inspect_database(executable, database, key):
    outputs = sql_query(
        executable,
        database,
        key,
        [
            "PRAGMA cipher_version;",
            "PRAGMA cipher_integrity_check;",
            "PRAGMA quick_check;",
            "PRAGMA user_version;",
            "SELECT hex(value_json) || '|' || value_version || '|' || updated_at FROM settings WHERE key='app';",
            "SELECT version || ':' || hex(checksum) FROM _sqlx_migrations ORDER BY version;",
        ],
    )
    cipher, encrypted_check, quick_check, schema, rows, migrations = outputs
    require(
        len(cipher) == 1 and cipher[0].startswith("4."),
        "必须使用 SQLCipher 4，不能使用普通 sqlite3。",
    )
    require(
        not encrypted_check and quick_check == ["ok"],
        "数据库完整性校验失败，停止修复。",
    )
    require(schema == ["6"], "脚本仅处理当前 schema 6，不运行数据库结构迁移。")
    require(len(rows) == 1, "没有唯一的 app 设置记录，停止修复。")
    encoded, version, updated = rows[0].split("|")
    require(version == "1", "外层 app 设置记录版本必须为 1。")
    raw = bytes.fromhex(encoded).decode("utf-8")
    require(len(raw.encode("utf-8")) <= 256 * 1024, "app 设置超过大小限制。")
    settings = parse_json(raw)
    require(
        isinstance(settings, dict)
        and type(settings.get("schemaVersion")) is int
        and settings["schemaVersion"] == 1,
        "外层 app.schemaVersion 必须为 1。",
    )
    return settings, raw, int(updated), migrations


def repaired_settings(settings):
    result = copy.deepcopy(settings)
    keymap = result.get("keymap")
    require(
        isinstance(keymap, dict) and set(keymap) == {"schemaVersion", "bindings"},
        "keymap 结构无效，脚本不会重置为默认快捷键。",
    )
    require(
        type(keymap["schemaVersion"]) is int and keymap["schemaVersion"] in (1, 2),
        "keymap 只允许从版本 1 或 2 修复，未来版本拒绝处理。",
    )
    bindings = keymap["bindings"]
    require(
        isinstance(bindings, list) and len(bindings) <= 128,
        "keymap.bindings 无效或超限。",
    )
    changes = []
    if keymap["schemaVersion"] != 2:
        keymap["schemaVersion"] = 2
        changes.append("keymap.schemaVersion: 1 → 2")
    ids = set()
    for index, binding in enumerate(bindings):
        path = f"keymap.bindings[{index}]"
        require(
            isinstance(binding, dict)
            and set(binding)
            == {
                "id",
                "commandId",
                "scope",
                "context",
                "trigger",
                "platform",
                "enabled",
            },
            f"{path} 字段无效。",
        )
        command = binding["commandId"]
        require(isinstance(command, str), f"{path}.commandId 类型无效。")
        if command in LEGACY:
            binding["commandId"] = LEGACY[command]
            changes.append(f"{path}.commandId: {command} → {binding['commandId']}")
        if binding["context"] == "workspace":
            binding["context"] = "workbench"
            changes.append(f"{path}.context: workspace → workbench")
        command, identity = binding["commandId"], binding["id"]
        require(command in COMMANDS, f"{path}.commandId 不属于当前已知动作。")
        require(
            isinstance(identity, str)
            and 0 < len(identity.encode()) <= 128
            and identity not in ids,
            f"{path}.id 无效或重复。",
        )
        ids.add(identity)
        terminal = command.startswith("terminal.")
        require(
            binding["scope"] == ("terminal" if terminal else "application")
            and binding["context"] == ("terminal" if terminal else "workbench"),
            f"{path} 的 scope/context 与动作不匹配。",
        )
        require(
            binding["platform"] == "all" and type(binding["enabled"]) is bool,
            f"{path} 的 platform/enabled 无效。",
        )
        trigger = binding["trigger"]
        require(isinstance(trigger, dict), f"{path}.trigger 无效。")
        modifiers = trigger.get("modifiers")
        require(
            isinstance(modifiers, list)
            and len(modifiers) <= 5
            and all(
                isinstance(m, str)
                and m in {"Primary", "Control", "Meta", "Alt", "Shift"}
                for m in modifiers
            ),
            f"{path}.trigger.modifiers 无效。",
        )
        if trigger.get("type") == "key":
            key = trigger.get("key")
            require(
                set(trigger) == {"type", "key", "modifiers"}
                and isinstance(key, str)
                and (bool(key) or not binding["enabled"])
                and len(key.encode()) <= 32
                and not any(unicodedata.category(c) == "Cc" for c in key),
                f"{path}.trigger.key 无效。",
            )
        else:
            require(
                set(trigger) == {"type", "button", "event", "modifiers"}
                and trigger.get("type") == "pointer"
                and trigger.get("button") == "primary"
                and trigger.get("event") == "click"
                and command == "terminal.openLink",
                f"{path}.trigger 鼠标动作无效。",
            )
    # 断言除快捷键版本、动作名与上下文外，其余内容完全保持，禁止顺便补默认项。
    restored = copy.deepcopy(result)
    restored["keymap"] = settings["keymap"]
    require(restored == settings, "修复范围检查失败。")
    return result, changes


def sql_text(text):
    # UTF-8 十六进制字面量避免引号、换行及任何用户内容参与 SQL 语法。
    return f"CAST(X'{text.encode('utf-8').hex()}' AS TEXT)"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--database",
        type=Path,
        default=Path.home()
        / "Library/Application Support/io.termeta/termeta-encrypted-v2.sqlite3",
    )
    parser.add_argument(
        "--sqlcipher", help="SQLCipher 4 可执行文件路径，默认从 PATH 或 Homebrew 查找"
    )
    parser.add_argument(
        "--apply",
        action="store_true",
        help="备份后实际写入；默认只在临时副本上检查和预览",
    )
    args = parser.parse_args()
    require(
        sys.platform == "darwin",
        "本独立脚本面向当前 macOS 本地库，其他平台不执行写入。",
    )
    os.umask(0o077)
    database = args.database.expanduser().absolute()
    metadata = database.with_suffix(".vault.json")
    files = [
        database,
        metadata,
        *(Path(str(database) + suffix) for suffix in ("-wal", "-shm", "-journal")),
    ]
    require(
        database.is_file() and metadata.is_file(),
        "数据库或配套 vault.json 不存在，请用 --database 指定正确路径。",
    )
    require(
        not database.with_suffix(".password-change").exists(),
        "存在改密恢复目录，需先由应用完成恢复，停止修复。",
    )
    executable = args.sqlcipher or shutil.which("sqlcipher")
    if not executable:
        executable = next(
            (
                p
                for p in (
                    "/opt/homebrew/bin/sqlcipher",
                    "/usr/local/bin/sqlcipher",
                    "/opt/homebrew/opt/sqlcipher/bin/sqlcipher",
                    "/usr/local/opt/sqlcipher/bin/sqlcipher",
                )
                if Path(p).is_file()
            ),
            None,
        )
    require(bool(executable), "未找到 SQLCipher 4，请先执行 brew install sqlcipher。")
    check_closed(files)
    initial = fingerprints(files)
    require(sys.stdin.isatty(), "请在交互终端中执行，密码不能通过管道传入。")
    key = unlock_key(metadata)
    # 预览只打开临时副本，避免 SQLite 的 WAL 恢复或共享内存影响原文件。
    with tempfile.TemporaryDirectory(prefix="termeta-keymap-preview-") as temporary:
        directory = Path(temporary)
        copy_files(files, directory)
        require(fingerprints(files) == initial, "复制期间原数据库发生变化，停止修复。")
        settings, raw, updated, migrations = inspect_database(
            executable, directory / database.name, key
        )
        repaired, changes = repaired_settings(settings)
    if not changes:
        print(
            "快捷键已经是版本 2 和 workbench 命名，无需写入。若应用仍报错，应定位其他设置字段。"
        )
        return
    print("计划修改：\n" + "\n".join(f"  {change}" for change in changes))
    if not args.apply:
        print("预览结束，原数据库未修改。确认后添加 --apply 再次执行。")
        return
    check_closed(files)
    require(fingerprints(files) == initial, "原文件在预览后发生变化，请重新执行。")
    backup = (
        database.parent
        / f"keymap-repair-backup-{datetime.now(timezone.utc):%Y%m%dT%H%M%SZ}-{uuid.uuid4().hex[:8]}"
    )
    backup.mkdir(mode=0o700)
    copy_files(files, backup)
    require(
        fingerprints([backup / path.name for path in files]) == initial,
        "备份哈希不一致，原数据库未修改。",
    )
    print(f"加密备份已保存：{backup}")
    check_closed(files)
    require(fingerprints(files) == initial, "备份期间原文件发生变化，停止写入。")
    encoded = json.dumps(
        repaired, ensure_ascii=False, separators=(",", ":"), allow_nan=False
    )
    # 条件更新与检查约束共同保证只改预览时的那一条记录；任何失败自动回滚。
    transaction = f"""
BEGIN IMMEDIATE;
CREATE TEMP TABLE repair_guard (ok INTEGER NOT NULL CHECK(ok = 1));
UPDATE settings SET value_json={sql_text(encoded)}, updated_at={int(time.time())}
WHERE key='app' AND value_version=1 AND value_json={sql_text(raw)} AND updated_at={updated};
INSERT INTO repair_guard VALUES (changes());
INSERT INTO repair_guard SELECT CASE WHEN count(*)=1 AND min(quick_check)='ok' THEN 1 ELSE 0 END FROM pragma_quick_check;
COMMIT;
"""
    sql_query(
        executable,
        database,
        key,
        ["PRAGMA synchronous = FULL;", transaction],
        readonly=False,
    )
    after, _raw, _updated, after_migrations = inspect_database(
        executable, database, key
    )
    require(
        after == repaired and after_migrations == migrations,
        f"写入后复核失败，请保留原文件与备份进一步检查：{backup}",
    )
    require(
        hashlib.sha256(metadata.read_bytes()).hexdigest() == initial[metadata.name],
        "元数据被其他进程修改，请保留备份检查。",
    )
    print(
        "修复完成：快捷键已持久化为版本 2，迁移校验和及配套元数据未变。现在可重新打开 Termeta。"
    )


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        # 不输出第三方异常原文或 traceback，避免把 SQL、密码和数据库内容带入日志。
        message = (
            str(error)
            if isinstance(error, RepairError)
            else "修复未完成，请检查依赖、路径、权限或数据库状态；原始备份应保留。"
        )
        print(f"错误：{message}", file=sys.stderr)
        sys.exit(1)
    except (KeyboardInterrupt, EOFError):
        print("已取消。若已创建备份，请保留备份目录。", file=sys.stderr)
        sys.exit(130)
