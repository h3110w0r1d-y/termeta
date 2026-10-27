# 本地快捷键一次性修复

这是独立的 macOS 维护脚本，不导入、编译或启动 Termeta，也不恢复应用内兼容逻辑。用于修复已经存在的 SQLCipher 格式 2、schema 6 本地库：仅将 `settings.key = "app"` 的 `keymap.schemaVersion` 改为 2，将六个已知 `workspace.*` 命令改为 `workbench.*`，将旧快捷键上下文改为 `workbench`。外层 app 版本保持 1。

绑定 ID、按键、鼠标修饰键、启用状态、顺序和多绑定均保留，不新增默认绑定。其他设置、主机、凭据、历史、工作区以及 `_sqlx_migrations` 均不修改。若快捷键已符合当前格式，脚本不写入；若是未知动作或其他无效快捷键字段，显示字段路径并停止，避免用默认配置掩盖问题。脚本不代替应用对其他设置字段的完整校验。

## 准备

1. **完全退出 Termeta，包括设置窗口。** 保持关闭直至脚本结束；脚本也会用 `lsof` 检查数据库占用。
2. 安装 SQLCipher 4，不能替换成系统 `sqlite3`：

   ```sh
   brew install sqlcipher
   ```

3. 选择一种 Python 依赖方式。已有 `uv` 时，可以直接使用下面的 `uv run` 命令；脚本头部声明了独立依赖，不修改项目依赖或锁文件。

   没有 `uv` 时，可创建独立虚拟环境：

   ```sh
   python3 -m venv /tmp/termeta-keymap-repair-venv
   /tmp/termeta-keymap-repair-venv/bin/python -m pip install 'argon2-cffi==25.1.0' 'cryptography==50.0.2'
   ```

脚本会隐藏输入当前保险库密码，按照项目相同的 Argon2id 参数派生密钥，并用配套 `vault.json` 的 AES-GCM 校验记录确认密码及文件配对。不会打印或保存密码/主密钥，不访问或修改系统钥匙串。不要把密码写入命令行、环境变量或聊天。Python 和第三方库的内存副本不保证可擦除。

独立依赖：[argon2-cffi 25.1.0](https://pypi.org/project/argon2-cffi/25.1.0/)（MIT）和 [cryptography 50.0.2](https://pypi.org/project/cryptography/50.0.2/)（Apache-2.0 或 BSD-3-Clause）。脚本使用其公开的 `hash_secret_raw` 和 `AESGCM` API；运行环境需 Python 3.10+，不修改项目依赖清单或锁文件。

## 预览

在仓库根目录执行，默认数据库为 `~/Library/Application Support/io.termeta/termeta-encrypted-v2.sqlite3`：

```sh
uv run scripts/repair-local-keymap.py
```

或使用前面建立的虚拟环境：

```sh
/tmp/termeta-keymap-repair-venv/bin/python scripts/repair-local-keymap.py
```

预览只打开临时加密副本，核验 SQLCipher/逻辑完整性，显示待变更的字段路径和内置动作名，不输出设置正文。原数据库及元数据不写入。

自定义路径或 SQLCipher 可执行文件时添加参数：

```sh
uv run scripts/repair-local-keymap.py --database '/实际路径/termeta-encrypted-v2.sqlite3' --sqlcipher /opt/homebrew/bin/sqlcipher
```

## 写入

确认预览范围正确后，保持 Termeta 关闭，加上 `--apply`：

```sh
uv run scripts/repair-local-keymap.py --apply
```

或：

```sh
/tmp/termeta-keymap-repair-venv/bin/python scripts/repair-local-keymap.py --apply
```

脚本在数据库旁创建独立的 `keymap-repair-backup-时间-随机值/`，包含原始加密数据库、配套元数据及现存 WAL/SHM/journal；目录权限为 0700，文件为 0600，复制后核对哈希。事务仅条件更新预览过的 app 记录；内容或时间戳已变化、锁冲突、检查失败时中止。提交后重新打开复核快捷键、完整性、迁移校验和及元数据。重复执行不会继续改写已完成的快捷键。

完成后再打开 Termeta。若仍出现设置校验错误，说明还需定位快捷键以外的设置项，不应继续重置或删除整库。

## 失败与恢复

- 密码错误、库文件不匹配、非 schema 6、数据库损坏、改密恢复目录存在或应用尚未退出，均停止处理。
- 若输出“加密备份已保存”后失败，保留该备份及当前数据库/元数据/侧文件，不要只恢复单个主文件。
- 需要恢复时，先完全退出应用，将当前数据库、元数据和现存侧文件整体移到另一目录保留，再把备份中的同名文件整组复制回原位置。不要将旧备份主文件与当前 WAL/SHM 混用。
- 此脚本不改数据库密钥、元数据或 SQLx 迁移，不处理改密恢复状态，也不绕过应用校验。
