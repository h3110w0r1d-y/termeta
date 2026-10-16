# Serial 串口终端

日期：2026-10-16。基础串口终端源码已落地。用户授权调试后，terminal-core/terminal-serial已在macOS定向编译、15项测试通过，ESP32-C3在用户指定tty节点以115200/8N1验证启动输出及关闭；未构建或启动桌面应用，不表示三平台或全界面验收通过。先前SerialConfig导入及平台cfg属性修复也已随这两个crate编译通过。

## 1. 入口与配置

主机页搜索框下按“新建主机”“快速连接”“本地终端”排列。前两项均使用悬浮NDropdown，下拉为SSH/Telnet/串口，Telnet仅禁用预留；独立串口按钮移除。新建主机→串口打开保存配置表单，快速连接→串口只启动临时会话（新建标签页footer也提供相同下拉），两种模式共用Naive UI串口配置弹窗，枚举系统设备，串口选择框右侧使用Ionicons RefreshOutline刷新图标按钮，无可见文字，保留“刷新串口”提示与无障碍名称，枚举或提交期间禁用；串口和波特率均为NAutoComplete可编辑输入框，选中后仍可修改、选择文本和复制，USB描述只显示在候选项中、不写入路径。macOS候选仅保留IOKit声明的IODialinDevice（tty）节点，不返回IOCalloutDevice（cu）节点；依据系统角色属性精确匹配，不检查设备路径前缀。Windows/Linux仍沿用库枚举结果。没有设备或枚举失败时仍可提交手输路径。设备在路径为空时默认选择第一项；新建/快速连接恢复默认配置，编辑/复制从保存项回填。保存模式增加必填名称、分组、标签、收藏和主机终端配色；枚举失败或加载中不阻断合法手输保存/连接。保存只提交配置，不要求设备在线，也不打开端口，不修改数据库schema。

| 字段 | 默认 | 可选范围 |
| --- | --- | --- |
| 串口 | 第一个可用端口 | 枚举候选或手输设备路径/COM名称，可附USB产品名 |
| 波特率 | 9600 | 常用预设或自填1–4000000整数，最终由驱动决定是否支持 |
| 字符集 | UTF-8 | UTF-8、GBK、GB18030、Big5、Shift_JIS、EUC-JP、EUC-KR、windows-1251、windows-1252、KOI8-R |
| Data bits | 8 | 8、7、6、5 |
| Stop bits | 1 | 1、2 |
| Flow control | None | None、RTS/CTS、XON/XOFF |
| Parity | No | No、Odd、Even |

字符集及以下参数位于默认折叠的“高级设置”。字符集保留NSelect；数据位、停止位、流控和校验使用Naive UI NRadioGroup/NRadioButton单选按钮组，直接绑定同一draft字段，保留数值/字符串类型及既有默认值。AppDialog新增autoFocus可选属性，默认true；串口弹窗显式设为false，打开时不自动聚焦表单或触发下拉菜单，保留手动点击、Tab与键盘选择。弹窗复用AppDialog的焦点陷阱、滚动正文及独立footer；NForm与控件直接共享reactive字段，连接前validate，枚举默认选择通过FormItem公开API重验。弹窗退场后恢复活动终端焦点。

参考用户指定Termius解包的defaultSerialOptions与串口表单选项，独立实现Vue/Rust代码；不执行或引入其React/Node代码。Termius中的DSR/DTR不在本轮可选项中：当前库仅提供None/RTS-CTS/XON-XOFF流控；手动DTR/RTS、信号观察及HEX发送仍属后续范围。

## 2. 类型与IPC

`list_serial_ports`仅允许已解锁main窗口调用，在阻塞任务中枚举，返回`{path, description}`数组。命令同步进入generate_handler、AppManifest命令名单与main capability；settings窗口没有此权限。

`open_session.target`增加`{source:"serial", config:{path, baudRate, charset, dataBits, stopBits, flowControl, parity}}`。flowControl为none/hardware/software，parity为none/odd/even，Rust枚举拒绝其他值；参数及字符集双端校验。枚举仅提供候选项，Rust打开不再依赖枚举成功或路径匹配。Unix要求绝对路径，规范化符号链接后目标须为/dev下的字符设备，再由serialport验证终端ioctl与参数，支持未枚举的设备及虚拟串口；不接受普通文件。Windows仅接受COM正整数端口名及其\\.\COM前缀形式，统一大小写与前导零。设备插拔竞态由系统打开错误处理，不缓存前端选择的设备句柄。

SessionConfig::Serial由terminal-serial::SerialFactory处理，复用20会话总限额、连接额度、owner、幂等、有界输入、Channel/ACK、心跳和窗口关闭。能力为resize=false、pty=false、binaryOutput=true、serialControl=false；后者不代表启动参数不可配置，而是当前没有会话内专用控制命令。Rust拒绝resize，前端仅本地fit，不发送远端resize。

保存串口沿用`{source:"saved",profileId}`，Rust依据主机kind加载完整serialConfig并派发SessionConfig::Serial；临时连接继续使用serial目标。HostDraft/HostProfile增加kind（ssh/serial，旧DTO缺失默认ssh）及可选serialConfig，串口SSH平坦字段必须为address/username空串、port=0、authPlan=[]，不接受password（即便空串）。配置存现有connection_profiles.kind与config_json.serialConfig，不创建ssh_profiles或认证记录，不改schema。禁止已有保存项变更类型，编辑、移动、收藏及复制配置保留全部参数。串口地址搜索匹配设备路径，卡片/列表/侧栏显示串口标签与Ionicons图标；系统识别仅限SSH。

历史摘要使用保存名称或临时端口名和`serial:<path>@<baudRate>`，保存串口携profileId，不记录正文或新增重放能力；与SSH/Local合计保留最近20次。保存配置与设备句柄分离，只有建立会话才占用设备。

## 3. 设备与字节流

独立terminal-serial crate依赖固定serialport 4.10.1及encoding_rs 0.8.42；macOS适配直接依赖既有传递依赖的固定core-foundation 0.10.1与io-kit-sys 0.4.1读取系统角色属性，不修改或复制串口库；core只含类型和验证，不依赖具体设备库。Linux禁用libudev默认feature，使用库的sysfs枚举；USB描述信息依系统可用性展示。Unix设备路径规范化后建立应用内独占登记，Windows使用规范化COM名称；macOS严格打开用户选择或输入的设备节点，仅规范化真实符号链接；不进行tty/cu互换或打开失败后的节点回退。枚举在macOS适配层按IOKit的IODialinDevice属性精确筛选串口库候选，保留库的USB描述，IOKit对象和CF属性以RAII释放，枚举失效报错；不按/dev/cu或/dev/tty前缀判断角色。Unix直接使用open_native默认的TIOCEXCL/flock独占，不重复set_exclusive；Windows通过CreateFile零共享模式限制重复打开。打开失败根据库错误类型提示占用、权限、缺失或参数问题，并附最多240字符、移除控制字符的驱动原因。设备线程持有登记直到系统句柄关闭，避免关闭后尚未回收就重复使用。

打开时通过库的dtr_on_open(false)明确DTR初始状态；None/Software流控下尽力设置RTS=false，Hardware流控下由驱动管理RTS，不手动覆盖。与串口库的DTR初始化策略一致，没有控制引脚的设备不会因该初始化失败而拒绝文本连接。该策略避免USB串口遗留的控制状态导致ESP32-C3无输出；不增加设备识别分支、打开重试或额外复位脉冲，也不切换tty/cu。控制线初始化会改变设备电平；本次设备实际显示USB_UART_CHIP_RESET及启动日志。手动控制DTR/RTS仍未开放。

读写各用阻塞任务，I/O超时100ms；读取桥4×8KiB，输入桥1块且保留核心字节预算。输出泵独立于输入与取消，驱动future丢弃时触发专用取消信号；队列接收端/发送端随之释放，唤醒等待队列的线程，设备I/O按超时退出。每块写入检查取消与5秒总预算，写超时或不确定交付终止连接，不自动重发，不调用可能无限等待的flush。异常驱动若不遵守系统超时，设备回收时限仍需真实平台验证；打开设备的底层系统调用不承诺可强行中断。

本轮提供文本终端：所有字符集（含UTF-8）的设备输出均在Rust使用encoding_rs流式解码为有效UTF-8后送xterm，非法序列显示U+FFFD（�）；避免xterm 6.0.0的字节解码器直接跳过非法UTF-8。跨读取块保留多字节状态，正常结束或设备错误排空时将未完成序列输出为�；用户取消仍可丢弃尚未排空的尾部。NUL、BEL、CR/LF和ESC等控制字节保留，由终端按原语义处理，不转换为可见符号。UTF-8输入仍直接传递字节，其他字符集将UTF-8输入流式转为目标编码；跨IPC块保留解码状态，无法表示的输入明确报错并停止连接，不静默发送HTML数字引用。输出帧及ACK偏移统计转码后的UTF-8字节，不代表物理串口字节数。未来HEX/原始捕获需另行定义原始字节通道，不能复用转码偏移声称无损捕获。

设备错误先停止读写，再排空已读队列与字符集尾部，沿用核心输出结束栅栏；用户取消仍优先响应。设备拔出、访问权限、占用、驱动拒绝参数和I/O错误使用脱敏错误；不承诺检测所有硬件FIFO溢出或每一种驱动丢字节。串口流控与应用背压不能保证外设永不溢出。

## 4. 终端与验证边界

串口终端复用配色、搜索、复制粘贴、稳定terminalId与原缓冲重开。TerminalTab独立保存kind和serialConfig展示元数据，saved目标仍只提交profileId；状态快照按后端kind同步，重连从当前主机元数据刷新展示参数。状态栏显示串口、波特率和字符集；不显示SFTP。复制标签/自动拆分建立第二条同端口连接被禁用，已有串口终端仍可在Workspace内移动或分屏布局；组广播仍须用户显式开启，逐目标应用串口编码，不自动重放。

实际静态检查见[路线记录](10-roadmap.md#2026-10-16serial-基础支持)。字符集跨块/不可表示字符/尾部、路径及参数白名单、owner/不支持resize回归已在本轮授权调试中执行通过，具体结果见下节；此前各轮静态检查不替代运行验证。运行验收仍需macOS/Windows/Linux设备枚举、手输未枚举路径/空列表/枚举失败、选中后修改与复制、符号链接及非法设备路径、macOS仅枚举系统dialin节点、手输tty/cu保持所填节点、重复连接、权限/占用、无设备/刷新、全部参数组合与驱动拒绝、USB拔插/重开、持续输出与无ACK时关闭、部分写入/流控阻塞、非UTF-8双向输入、切页/分屏保活及窗口关闭回收。DTR/RTS控制、DSR/DTR流控、HEX/Local Echo未实现，EXT-02保持部分完成。


## 5. 本次授权实机诊断

设备：用户指定的`/dev/tty.usbmodem11101`，USB JTAG/serial debug unit（VID 303a/PID 1001），115200、8N1、None流控、UTF-8。原Rust驱动两次正常进入Connected但收取0字节；同一串口显式解除初始DTR/RTS后可收到启动输出。修复后两次经SerialFactory→SessionManager→二进制帧→消费ACK的真实链路，各收到2965字节（UTF-8输出规范化修复前的接收量，含ESP-ROM、ESP-IDF v6.1 bootloader及应用启动日志），随后正常关闭；未发送应用数据、切换cu节点或运行桌面应用。诊断期间用户允许的控制线试验触发了开发板重启，不修改设备固件或闪存。

显式接收诊断入口为[serial_monitor](../crates/terminal-serial/examples/serial_monitor.rs)，须在用户授权真实设备调试后运行：

```sh
cargo run --locked --offline -p terminal-serial --example serial_monitor -- /dev/tty.usbmodem11101 115200 5
```

该工具复用实际串口打开策略，不发送应用数据，控制线初始化可能触发设备开机输出；观察时间1–30秒，预览最多16KiB，输出按转义文本展示，不执行设备控制序列。`cargo test --locked --offline -p terminal-serial -p terminal-core --lib`现共15项通过，覆盖UTF-8非法序列替换/任意读取边界/单字节读取/截断尾部/控制字节保留/输入原样传递，以及既有其他字符集、设备路径、参数及owner/resize回归。使用已安装xterm 6.0.0的Terminal在Node中写入同一构造数据并读取缓冲：原始字节为GP@KS，规范化UTF-8后为GP�@�K�S�，ANSI序列不作为正文显示。Rust格式和差异静态检查通过；本次解码修复未重新连接设备。桌面Channel/xterm显示、全部高级参数、硬件流控、其他设备和Windows/Linux仍待验收。

2026-10-16：串口保存主机及类型下拉仅完成静态类型/结构/格式、SQL模型、资源与文档差异核对，新增存储回归源码未执行。此前15项串口测试及ESP32-C3证据不覆盖本次持久化/多类型主机集成；保存后重启、编辑/复制/收藏/移动、旧SSH数据兼容、saved串口重开与SFTP隔离、菜单/焦点/搜索及三平台仍待运行验收。
