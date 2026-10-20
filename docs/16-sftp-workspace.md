# SFTP 浏览、上传下载与双栏对拷

更新：2026-10-21。本文维护连接、文件授权和传输边界；当前范围与完整验收状态见 [产品需求](01-product-requirements.md#8-当前实现与限制)。

## 1. 连接与生命周期

终端侧栏仅适用于单个已连接 SSH 终端，复用其已认证传输的独立 SFTP channel。开启侧栏时禁止作为分屏来源或目标；隐藏保留任务，断开或关闭终端取消任务，重连不重放。

固定 SFTP 页为双远端文件浏览器，左右从保存 SSH 主机各建独立连接，可连接同一主机。复用主机信任与认证，不借用终端、不申请 PTY/Shell。文件专用 manager 最多两条连接；切页保留，断开或窗口退出关闭。通用共享连接租约、本地浏览和其他侧栏停靠位置尚未实现。

终端、文件与转发共用前端轻量控制投影和认证队列，连接资源仍由各自 Rust manager 持有。core 在驱动结束或关闭请求时通知壳撤销 SFTP 浏览器，取消传输、选择器/覆盖确认的应用等待及原生拖出，定时扫描只兜底。浏览器创建与传输登记继承会话取消，迟到登记和原生回调不得恢复操作；系统选择器自身可能仍可见，迟到选择结果不再消费。

## 2. 目录与路径

每页最多 100 条，单视图最多 10000 条，SFTP 报文上限 256 KiB。支持路径输入、逻辑上级、主目录、刷新、目录优先排序及全局隐藏文件偏好；`..` 不计入分页和选择。

READLINK 获取链接目标，STAT 尽力判断目标类型，失败不阻断列表。可进入目录链接；浏览保留逻辑绝对路径，折叠重复斜杠、`.`、`..`，不越过根。相对输入通过 REALPATH 获取起始目录。

当前只接受安全的 UTF-8 单个名称，不支持任意字节名、控制字符或反斜线名；不得以有损显示名称执行传输。远端访问范围以 SSH 账户权限为界。

## 3. 上传与覆盖

按钮通过原生选择器授权普通文件；终端侧栏原生拖入支持文件和目录，目标为当前浏览目录。Rust 签发绑定 owner 的一次性 dropId，前端不提交任意本地路径。授权保留 5 分钟、最多 16 批，每批最多 1000 个根路径；仅接受真实外部进入后的 Drop，应用内拖放或拖出回流不产生上传授权。

目录预扫描最多 64 层、10000 项，不跟随本地链接、不接受特殊文件。Windows 同样拒绝 junction、符号链接及其他 reparse point，打开上传文件时使用不跟随末级重解析点的标志并复核已打开对象。已有目录合并，文件/目录冲突拒绝，上传树内链接不自动递归合并。已进入的远端目录链接可作为目标目录。

同名普通文件逐项确认覆盖。新文件独占创建；已有文件打开后 FSTAT 核对类型，再截断写入。无远端临时文件或回滚；失败、取消或断网可能留下部分内容。SFTPv3 不提供通用 NOFOLLOW/条件写入，不能消除并发路径替换竞态或承诺原子覆盖。

## 4. 下载与原生拖出

下载按钮仅支持单个普通文件，通过原生保存对话框授权目标。先写同目录独占临时文件（Unix 权限 0600），完成后以 hard_link 无覆盖发布；只清理本任务的精确临时路径，清理失败报告路径。

macOS 支持最多 100 个普通文件/真实目录拖到 Finder，通过 NSFilePromiseProvider 按需下载。Finder 请求兑现后才创建任务；无效落点、Esc 或应用内松开取消手势，不留下下载记录。成功要求 Copy 会话结束且全部承诺兑现；收尾拒绝迟到请求，内部落点不能转为上传。

目录递归独立于列表游标，最多 64 层、10000 项；不跟随链接或特殊文件，只创建 Finder 授权的新目录，不覆盖已有目标。失败保留已完成部分。链接拖出、Linux 原生拖出及按钮目录下载尚未实现。

Windows 终端侧栏支持最多 100 个普通文件/真实目录混合拖到资源管理器，包括嵌套目录、空目录和空文件。使用 Shell `FILEGROUPDESCRIPTORW` / `FILECONTENTS`：目录显式标记为目录属性，父目录在子项之前；内容索引与描述符索引一致。接收端请求时才生成有界清单，最多 64 层、合计 10000 项，独立于浏览器分页，不预下载到临时文件。

每次内容请求及克隆使用独立游标，支持定位、只读流复制和大小查询；远端打开后复核普通文件类型与句柄大小。64 KiB 分块，进度节流，只记录已交付的字节区间；最多 128 个存活流/等待回调，区间跟踪有总量限制。Windows 设备名、尾随空格/句点、非法字符、大小写冲突及超过描述符容量的相对路径明确拒绝，不静默改名；链接、特殊文件和无法获取大小的文件不拖出。

OLE 松键不代表传输完成：异步目标的 `EndOperation`、Copy 结果与文件交付状态共同决定完成；空目录由目录描述符和目标成功结果确认。接收端接受前的描述符查询不显示下载任务；无效落点、Esc 和应用内落点取消，不产生上传授权。取消/断线后阻断迟到打开与读取，排空在途 I/O 并关闭远端句柄，再在创建它们的 STA 线程释放原生保活引用。同步 COM 回调将远端请求交给后台，等待期间继续分派窗口/COM 消息，同一流的重入操作拒绝阻塞。

Windows 外部接收端负责创建目标、冲突交互和部分文件清理；源端只提供 Copy，不删除远端源，不承诺外部目标的无覆盖发布或失败回滚。目录预扫描不构成远端快照，不能普遍消除 SFTPv3 并发路径替换；本地上传同样不承诺完整目录树的并发替换防护。Windows 实际拖入/拖出和其他接收端兼容性仍待运行验收。

## 5. 双远端对拷

可把选中的普通文件/真实目录复制到对侧当前目录，支持栏间拖放；双栏不提供系统文件拖入/拖出。手势取消不创建任务。

`sftp_copy` 重查两端文件会话 owner 和连接，获取两端操作锁及全局传输许可；Rust 流式中转，不落盘、不经 WebView。每批最多 100 根，扫描上限同目录上传；目录合并、链接拒绝和逐文件覆盖确认沿用上传语义。取消/失败不回滚，可能留下部分文件。

Rust 的内部 EndpointIdentity 使用规范 host、port 和 username，不使用省略端口的显示标题，也不因共享主机密钥断言同一文件命名空间。同端点使用 REALPATH 拒绝复制到自身或其子树；不同主机别名、硬链接和并发路径替换无法普遍证明对象身份，不承诺严格快照。端点身份不改变显示标题或公开会话 DTO。传输期间禁止两栏导航或切换连接，重连不恢复任务。

## 6. 权限、资源与源码

文件正文仅在 Rust 流动，不进入 WebView、状态仓库、数据库或日志。64 KiB 数据块，全局最多 4 个传输，每浏览器一个操作；进度经 Channel 节流，u64 字节数以十进制字符串传递。没有持久化队列、断点续传、完整批量冲突策略或哈希校验。

命令为 `sftp_connect`、`sftp_list`、`sftp_copy`、`sftp_transfer`、`sftp_drop_upload`、`sftp_start_drag`、`sftp_cancel`；均需命令 ACL、main 窗口及会话 owner 校验，本地路径只能来自原生授权。

源码入口：[文件命令](../src-tauri/src/commands/files.rs)、[独立连接与对拷](../src-tauri/src/commands/file_workspace.rs)、[文件服务](../src-tauri/src/files/)、[terminal-files](../crates/terminal-files/src/lib.rs)、[terminal-sftp](../crates/terminal-sftp/src/lib.rs)、[macOS 拖出](../src-tauri/src/file_drag_macos.rs)、[Windows 拖出](../src-tauri/src/file_drag_windows.rs)。

## 7. 验证与资料

历史仅有用户确认 macOS 原生拖出取消后交互恢复的局部证据；不能外推为 Finder 全矩阵、双栏对拷或三平台通过。编译、真实 SSH、文件冲突/中断、无 ACK 关闭及原生拖放仍待完整验收，统一见 [运行验收边界](01-product-requirements.md#11-运行验收边界)。

参考：[Windows Shell 数据格式](https://learn.microsoft.com/en-us/windows/win32/shell/clipboard)、[FILEDESCRIPTORW](https://learn.microsoft.com/en-us/windows/win32/api/shlobj_core/ns-shlobj_core-filedescriptorw)、[IDataObjectAsyncCapability](https://learn.microsoft.com/en-us/windows/win32/api/shldisp/nn-shldisp-idataobjectasynccapability)、[russh-sftp 3.0.1](https://docs.rs/russh-sftp/3.0.1/russh_sftp/)、[NSFilePromiseProvider](https://developer.apple.com/documentation/appkit/nsfilepromiseprovider)。
