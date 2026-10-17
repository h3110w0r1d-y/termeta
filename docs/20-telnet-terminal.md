# Telnet 文本终端

更新：2026-10-18。TCP 与基础 Telnet 协商、保存及临时连接已有源码；真实服务兼容性和三平台验收尚未完成。

## 1. 配置与安全边界

支持域名、IPv4、无方括号 IPv6，端口单填，默认 23、范围 1–65535。保存使用既有主机管理，不访问网络；已有保存项不能原地转换类型。

Telnet 明文传输，用户名和密码在远端提示中输入；不读取 SSH 凭据，不保存登录脚本，远端负责回显及密码隐藏。当前只提供 UTF-8 显示，无 Local Echo、TLS/STARTTLS、自动登录、字符集选择、系统探测或 SFTP。

权威 `TelnetConfig` 仅含 `configVersion:1`、`address`、`port`，拒绝未知或 SSH 认证字段。临时目标使用 `{source:"telnet",config}`；保存目标仅传 `{source:"saved",profileId}`，Rust 重读配置。持久化及迁移统一见 [存储](07-storage.md)，不在协议文档复制 schema 版本表。

## 2. 协议行为

| 能力 | 行为与限制 |
| --- | --- |
| IAC | 跨 TCP 块保留解析状态；IAC IAC 为正文 255，发送时转义 255；协议命令不进入正文 |
| BINARY | 收发方向独立协商，启用后保留对应方向控制字节，仍处理 IAC |
| ECHO / SGA | 接受服务端回显、本地拒绝回显；支持逐字节输入，不额外添加 Local Echo |
| TTYPE | 协商同意后对 SEND 回复 `xterm-256color` |
| NAWS | 服务端 DO 后发送最新尺寸，DONT 后停止；16 位网络序并转义 255 |
| NVT | 接收 CR NUL 为 CR、保留 CR LF；发送 CR/LF 为 CR LF，跨块 CR LF 不重复；BINARY 方向不转换 |
| 其他选项 | WILL 回 DONT，DO 回 WONT；已达成的重复协商不循环确认 |
| 子协商 | 跨块解析并剥离，最多 4096 字节；非法或超限以协议错误结束 |

初始化请求本地 BINARY/SGA/TTYPE/NAWS 和远端 BINARY/ECHO/SGA，每方向保留协商状态。协商响应与用户输入经同一有界队列发送，避免字节交错。

不实现 LINEMODE、本地行编辑、环境变量、压缩、认证/加密扩展或 URG/TCP 同步。依赖这些能力或非 UTF-8 编码的旧设备不在当前兼容承诺内。

## 3. 生命周期与资源

TCP 建立即 Connected，登录提示不产生认证挑战。正常 EOF 为 Closed，已连接后的网络/协议错误为 Disconnected；失败或断线保留终端，手动重连建立新 SessionId，见 [会话模型](04-session-model.md)。

DNS/TCP 共用 30 秒截止，单次 socket 写入也限 30 秒，均可取消；读取块 8 KiB、写队列 8 项，输入及输出继续受 core 字节预算限制。取消可中止建连、写入、队列和无 ACK 输出等待，驱动释放即关闭 TCP。帧及 ACK 仅统计剥离协议后的正文。

能力为 `resize=true`、`pty=false`、`binaryOutput=true`、`serialControl=false`；resize 表示 NAWS 通知，不能保证服务器接受。支持独立复制连接、分屏与广播，各连接手动登录，不共享或重放输入。

## 4. 源码与验证边界

入口：[配置校验](../crates/terminal-core/src/telnet.rs)、[TCP 驱动](../crates/terminal-telnet/src/lib.rs)、[流式协议](../crates/terminal-telnet/src/protocol.rs)、[会话命令](../src-tauri/src/commands/sessions.rs)。

协议分片、协商、NVT/BINARY、NAWS、限额及 TCP 取消有回归源码，尚未执行完整编译/运行验收。真实设备登录/回显、全屏与尺寸变化、恶意协商、慢消费者取消、重连资源回收和加密库集成仍待验证，见 [运行验收边界](01-product-requirements.md#11-运行验收边界)。
