# Serial 串口终端

更新：2026-10-27。基础文本终端与保存配置已有源码；历史 macOS 局部验证不覆盖完整桌面集成及三平台验收。

## 1. 配置与范围

支持保存配置或临时连接；保存不要求设备在线，也不打开设备或触发控制线。枚举提供候选，允许手输；实际打开保持用户指定的节点，不依赖枚举成功。

| 参数 | 默认 | 支持范围 |
| --- | --- | --- |
| 波特率 | 9600 | 1–4000000 整数，最终由驱动决定 |
| 字符集 | UTF-8 | UTF-8、GBK、GB18030、Big5、Shift_JIS、EUC-JP、EUC-KR、windows-1251、windows-1252、KOI8-R |
| 数据位/停止位 | 8 / 1 | 5–8 / 1、2 |
| 流控 | None | None、RTS/CTS、XON/XOFF |
| 校验 | No | No、Odd、Even |

手动 DTR/RTS、信号观察、DSR/DTR 流控、HEX 与 Local Echo 尚未实现。无 SFTP 或远端 resize；现有串口窗格可参与布局和显式广播，但不通过复制标签/自动拆分建立同端口连接。

## 2. 配置与权限边界

`list_serial_ports` 仅供已解锁 main 窗口调用，在阻塞任务枚举 `{path, description}`。临时会话使用 SerialConfig；保存项只传 ProfileId，由 Rust 重读权威配置。配置不含 SSH 认证字段或凭据，已有保存项不能原地变更类型。

SessionFactory 复用统一 owner、幂等、配额、取消、输出/ACK 和窗口清理；能力为 `resize=false`、`pty=false`、`binaryOutput=true`、`serialControl=false`。后者表示没有会话内控制命令，启动参数仍可配置。

Unix 路径须为绝对路径，规范化符号链接后是 `/dev` 下的字符设备；Windows 只接受 COM 正整数名称及其设备路径形式，并统一大小写和前导零。不能打开普通文件，设备插拔竞态由系统打开错误处理。

## 3. 设备与字节流

使用 serialport 与 encoding_rs，平台依赖见 [依赖预研](02-library-research.md)。macOS 枚举按 IOKit `IODialinDevice` 角色筛选，实际打开不进行 tty/cu 互换或失败回退；Linux 禁用 libudev 默认 feature。应用内登记与 OS 独占同时生效，登记直到设备句柄全部关闭才释放。

打开时 DTR 初始化为 false；None/Software 流控下尽力设置 RTS=false，Hardware 模式由驱动管理 RTS。初始化可能导致设备重启或启动输出，不增加设备特例和额外复位脉冲；不支持控制线的设备仍可建立文本连接。

阻塞读写使用 100ms I/O 超时、有界读取桥和单块输入桥，取消独立于输入及输出 ACK；单块写入总预算 5 秒，交付不确定即停止，不重发、不调用可能无限等待的 flush。系统打开调用和异常驱动不承诺可强行中断，设备回收时限需平台实测。

所有设备输出均流式解码为有效 UTF-8，非法或结束时未完成的序列显示 U+FFFD；保留 NUL、BEL、CR/LF、ESC 等控制字节语义。UTF-8 输入直传，其他字符集流式转换，无法表示的输入报错并结束连接，不静默替换。帧和 ACK 统计转码后的字节，不代表物理串口字节数；未来 HEX/原始捕获需要独立通道。

设备错误先停读写，再排空已读数据和转码尾部；用户取消允许放弃尾部。错误脱敏，不能承诺检测所有硬件 FIFO 溢出；应用背压和硬件流控不能保证外设永不丢字节。

## 4. 源码与待验收

入口：[参数校验](../crates/terminal-core/src/serial.rs)、[SerialFactory](../crates/terminal-serial/src/lib.rs)、[设备路径](../crates/terminal-serial/src/device_path.rs)、[字符集](../crates/terminal-serial/src/codec.rs)、[macOS 枚举](../crates/terminal-serial/src/macos.rs)。通用生命周期见 [会话模型](04-session-model.md)。

三平台枚举/手输/权限/占用、控制线及高级参数、插拔重开、字符集双向输入、阻塞取消和持久化集成仍待运行验收。

## 5. 已有验证与诊断入口

2026-10-16 历史授权调试在 macOS 定向编译 terminal-core/terminal-serial，15 项测试通过；指定 ESP32-C3 以 115200/8N1、None 流控经真实会话与 ACK 链路验证启动输出和正常关闭。xterm 无界面缓冲对比验证非法 UTF-8 替换；该修复后未重新连接设备。

未构建或启动完整桌面应用；证据不覆盖后续保存配置集成、其他设备/参数、硬件流控或 Windows/Linux。真实设备诊断工具 [serial_monitor](../crates/terminal-serial/examples/serial_monitor.rs) 仅接收并转义预览数据，须用户明确授权后运行；授权边界见 [验证说明](../tests/README.md)。
