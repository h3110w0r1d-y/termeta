# Serial 串口终端

更新：2026-10-16。基础文本终端及保存配置已实现。已有 macOS 局部编译/测试和指定设备证据，不覆盖完整桌面应用、后续持久化集成或三平台验收。

## 1. 入口与配置

主机页“新建主机/快速连接 → 串口”分别保存配置或启动临时会话，新建标签页 footer 也提供快速连接。类型菜单包含已启用的 SSH/Telnet/串口，无独立串口按钮。保存模式补名称/分组/标签/收藏/配色，不要求设备在线、不打开设备，也不触发控制线。

串口和波特率用 NAutoComplete，可选择、手输、修改/复制；描述仅展示，不写入路径。刷新按钮位于串口输入右侧，图标有提示/无障碍名称。路径为空时可选第一候选，枚举失败/无设备仍允许有效手输。macOS 依据 IOKit IODialinDevice 角色筛选候选，Windows/Linux 使用库枚举；实际打开保持用户指定节点。

| 字段         | 默认           | 可选范围                                                                                 |
| ------------ | -------------- | ---------------------------------------------------------------------------------------- |
| 串口         | 第一个可用端口 | 枚举候选或手输设备路径/COM名称，可附USB产品名                                            |
| 波特率       | 9600           | 常用预设或自填1–4000000整数，最终由驱动决定是否支持                                      |
| 字符集       | UTF-8          | UTF-8、GBK、GB18030、Big5、Shift_JIS、EUC-JP、EUC-KR、windows-1251、windows-1252、KOI8-R |
| Data bits    | 8              | 8、7、6、5                                                                               |
| Stop bits    | 1              | 1、2                                                                                     |
| Flow control | None           | None、RTS/CTS、XON/XOFF                                                                  |
| Parity       | No             | No、Odd、Even                                                                            |

高级设置默认折叠，字符集 NSelect，其余参数 NRadioGroup/NRadioButton。弹窗关闭自动聚焦，保留手动/键盘与焦点陷阱；NForm 与控件共享响应式字段，保存/连接前校验。新建恢复默认，编辑/复制回填保存参数。手动 DTR/RTS、信号观察、DSR/DTR 流控与 HEX 尚未实现。

## 2. 类型与IPC

`list_serial_ports`仅允许已解锁main窗口调用，在阻塞任务中枚举，返回`{path, description}`数组。命令同步进入generate_handler、AppManifest命令名单与main capability；settings窗口没有此权限。

`open_session.target`增加`{source:"serial", config:{path, baudRate, charset, dataBits, stopBits, flowControl, parity}}`。flowControl为none/hardware/software，parity为none/odd/even，Rust枚举拒绝其他值；参数及字符集双端校验。枚举仅提供候选项，Rust打开不再依赖枚举成功或路径匹配。Unix要求绝对路径，规范化符号链接后目标须为/dev下的字符设备，再由serialport验证终端ioctl与参数，支持未枚举的设备及虚拟串口；不接受普通文件。Windows仅接受COM正整数端口名及其\\.\COM前缀形式，统一大小写与前导零。设备插拔竞态由系统打开错误处理，不缓存前端选择的设备句柄。

SessionConfig::Serial由terminal-serial::SerialFactory处理，复用20会话总限额、连接额度、owner、幂等、有界输入、Channel/ACK、心跳和窗口关闭。能力为resize=false、pty=false、binaryOutput=true、serialControl=false；后者不代表启动参数不可配置，而是当前没有会话内专用控制命令。Rust拒绝resize，前端仅本地fit，不发送远端resize。

保存串口沿用`{source:"saved",profileId}`，Rust依据主机kind加载完整serialConfig并派发SessionConfig::Serial；临时连接继续使用serial目标。HostDraft/HostProfile 共享 kind 与可选 serialConfig，旧 DTO 缺失 kind 默认 ssh；当前列表还包含 Telnet/workspace，串口SSH平坦字段必须为address/username空串、port=0、authPlan=[]，不接受password（即便空串）。配置存现有connection_profiles.kind与config_json.serialConfig，不创建ssh_profiles或认证记录，不改schema。禁止已有保存项变更类型，编辑、移动、收藏及复制配置保留全部参数。串口地址搜索匹配设备路径，卡片/列表/侧栏显示串口标签与Ionicons图标；系统识别仅限SSH。

历史摘要使用保存名称或临时端口名和`serial:<path>@<baudRate>`，保存串口携profileId，不记录正文或新增重放能力；与 SSH/Telnet/Local 合计保留最近20次。保存配置与设备句柄分离，只有建立会话才占用设备。

## 3. 设备与字节流

独立terminal-serial crate依赖固定serialport 4.10.1及encoding_rs 0.8.42；macOS适配直接依赖既有传递依赖的固定core-foundation 0.10.1与io-kit-sys 0.4.1读取系统角色属性，不修改或复制串口库；core只含类型和验证，不依赖具体设备库。Linux禁用libudev默认feature，使用库的sysfs枚举；USB描述信息依系统可用性展示。Unix设备路径规范化后建立应用内独占登记，Windows使用规范化COM名称；macOS严格打开用户选择或输入的设备节点，仅规范化真实符号链接；不进行tty/cu互换或打开失败后的节点回退。枚举在macOS适配层按IOKit的IODialinDevice属性精确筛选串口库候选，保留库的USB描述，IOKit对象和CF属性以RAII释放，枚举失效报错；不按/dev/cu或/dev/tty前缀判断角色。Unix直接使用open_native默认的TIOCEXCL/flock独占，不重复set_exclusive；Windows通过CreateFile零共享模式限制重复打开。打开失败根据库错误类型提示占用、权限、缺失或参数问题，并附最多240字符、移除控制字符的驱动原因。设备线程持有登记直到系统句柄关闭，避免关闭后尚未回收就重复使用。

打开时通过库的dtr_on_open(false)明确DTR初始状态；None/Software流控下尽力设置RTS=false，Hardware流控下由驱动管理RTS，不手动覆盖。与串口库的DTR初始化策略一致，没有控制引脚的设备不会因该初始化失败而拒绝文本连接。控制线初始化可能触发设备重启/启动输出，不增加设备特例、额外复位脉冲或节点切换。手动控制DTR/RTS仍未开放。

读写各用阻塞任务，I/O超时100ms；读取桥4×8KiB，输入桥1块且保留核心字节预算。输出泵独立于输入与取消，驱动future丢弃时触发专用取消信号；队列接收端/发送端随之释放，唤醒等待队列的线程，设备I/O按超时退出。每块写入检查取消与5秒总预算，写超时或不确定交付终止连接，不自动重发，不调用可能无限等待的flush。异常驱动若不遵守系统超时，设备回收时限仍需真实平台验证；打开设备的底层系统调用不承诺可强行中断。

当前提供文本终端：所有字符集（含UTF-8）的设备输出均在Rust使用encoding_rs流式解码为有效UTF-8后送xterm，非法序列显示U+FFFD（�）；避免xterm 6.0.0的字节解码器直接跳过非法UTF-8。跨读取块保留多字节状态，正常结束或设备错误排空时将未完成序列输出为�；用户取消仍可丢弃尚未排空的尾部。NUL、BEL、CR/LF和ESC等控制字节保留，由终端按原语义处理，不转换为可见符号。UTF-8输入仍直接传递字节，其他字符集将UTF-8输入流式转为目标编码；跨IPC块保留解码状态，无法表示的输入明确报错并停止连接，不静默发送HTML数字引用。输出帧及ACK偏移统计转码后的UTF-8字节，不代表物理串口字节数。未来HEX/原始捕获需另行定义原始字节通道，不能复用转码偏移声称无损捕获。

设备错误先停止读写，再排空已读队列与字符集尾部，沿用核心输出结束栅栏；用户取消仍优先响应。设备拔出、访问权限、占用、驱动拒绝参数和I/O错误使用脱敏错误；不承诺检测所有硬件FIFO溢出或每一种驱动丢字节。串口流控与应用背压不能保证外设永不溢出。

## 4. 终端与验证边界

复用配色、搜索、复制粘贴、稳定 terminalId 与原缓冲重开；状态栏展示串口/波特率/字符集。saved 目标只传 profileId，Rust 重读配置，前端 kind/serialConfig 仅作展示，不参与授权。

无 SFTP/远端 resize，禁止复制标签/自动拆分新建同端口连接；现有串口窗格仍可参与布局/迁移，广播须显式开启并按目标编码，不重放输入。

待验收包括三平台枚举/手输/路径/权限/占用、macOS 角色候选与所填节点保持、全部高级参数、USB 插拔/重开、流控阻塞/持续输出/无 ACK 关闭、非 UTF-8 双向输入、窗口回收、分屏保活，以及保存后重启/编辑/复制/收藏/移动和多类型集成。EXT-02 保持部分完成，HEX/Local Echo 属于 EXT-03。

## 5. 已有验证与诊断入口

2026-10-16 历史授权调试：macOS 定向编译 terminal-core/terminal-serial，15项测试通过，覆盖编码边界/非法 UTF-8/控制字节、设备路径/参数与 owner/resize。指定 ESP32-C3 USB JTAG/serial 设备经真实 SerialFactory → SessionManager → 输出帧 → ACK 链路，以115200/8N1、None流控验证启动输出及正常关闭；初始化控制线可能触发开发板重启。xterm 6 无界面缓冲对比验证非法序列替换，解码修复后未重新连接设备。

未构建或启动桌面应用，以上不覆盖后续保存主机集成、其他设备/参数、硬件流控或 Windows/Linux。新增存储回归源码未执行，不能把局部结果当整项验收。

[serial_monitor](../crates/terminal-serial/examples/serial_monitor.rs) 为显式接收诊断工具，须用户授权真实设备调试后运行：

```sh
cargo run --locked --offline -p terminal-serial --example serial_monitor -- /dev/tty.usbmodem11101 115200 5
```

工具使用真实打开策略，不发送应用数据；观察1–30秒，预览最多16 KiB，转义显示、不执行控制序列。测试入口和授权边界见 [测试说明](../tests/README.md)。
