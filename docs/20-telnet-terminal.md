# Telnet 文本终端

日期：2026-10-16。基础源码已实现，尚未编译、启动或连接实际Telnet服务；不代表三平台或协议兼容性已验收。

## 1. 范围与入口

主机页“新建主机”和“快速连接”以及新建标签页的快速连接菜单启用Telnet。默认23端口，可修改为1–65535；地址支持域名、IPv4及不带方括号的IPv6，端口单独填写。保存主机使用现有名称、分组、收藏、标签、配色、搜索、右键编辑/复制与批量连接；保存不访问网络，现有记录的类型不可原地转换。

用户名与密码在远端终端提示中输入，不读取SSH凭据、不保存自动登录脚本。表单和状态栏标识明文传输；头像采用现有Ionicons的GlobeOutline。显示使用UTF-8，远端负责回显及密码隐藏，应用不添加Local Echo。当前无TLS/STARTTLS、自动登录、字符集选择、SSH系统探测或SFTP。

## 2. 分层与配置契约

- `crates/terminal-core/src/telnet.rs`：TelnetConfig与地址/端口校验，不依赖TCP、Tauri或数据库。
- `crates/terminal-telnet/src/protocol.rs`：流式IAC/NVT/选项状态、转义及子协商。
- `crates/terminal-telnet/src/lib.rs`：Tokio TCP工厂与受界双向I/O，不调用系统telnet或Shell程序。
- `src-tauri/src/commands/sessions.rs`：保存目标从Rust读取权威配置，临时目标使用typed TelnetConfig，复用现有命令、ACL和解锁/owner校验。
- `src/modules/hosts/HostEditor.vue`：共用Naive UI表单，隐藏SSH认证字段；工作区继续按terminalId持有xterm。

```ts
interface TelnetConfig {
  configVersion: 1;
  address: string;
  port: number;
}
// 快速连接
const target = {
  source: "telnet",
  config: { configVersion: 1, address: "192.0.2.1", port: 23 },
};
// 保存主机连接仍仅传 { source: "saved", profileId }
```

TelnetConfig拒绝未知字段。HostDraft/Profile沿用平坦address/port，kind=telnet时username为空、authPlan=[]、serialConfig为空，拒绝SSH认证引用及任何顶层password参数。数据库使用config_json.telnetConfig保存版本/地址/端口，不创建ssh_profiles或profile_auth_steps；保存、收藏、迁移与复制均保留原参数和revision语义。

SessionConfig增加Telnet分支，SessionSnapshot.kind与available_session_kinds包含telnet。能力为resize=true、pty=false、binaryOutput=true、serialControl=false；resize表示NAWS通知，不表示远端一定接受。IPC仍为版本1，输出帧/ACK偏移按剥离Telnet协议后的正文统计。

## 3. 协议行为

| 行为       | 实现边界                                                                                 |
| ---------- | ---------------------------------------------------------------------------------------- |
| TCP        | DNS及建连共用30秒截止，支持独立取消；socket使用TCP_NODELAY                               |
| IAC        | 跨TCP块保留Data/Command/Option/Sub/SubCommand状态；IAC IAC解码为正文255，发送时转义255   |
| BINARY (0) | 双向分别协商，成功后对应方向保留控制字节，仍处理IAC                                      |
| ECHO (1)   | 接受服务端WILL ECHO，本地拒绝DO ECHO，不额外回显键入内容                                 |
| SGA (3)    | 双向支持，逐字节输入；GA等单字节控制命令不进入正文                                       |
| TTYPE (24) | 服务端同意后对SEND回复IS xterm-256color                                                  |
| NAWS (31)  | 服务端DO后发送最新宽高；16位网络字节序，子协商内容中的255转义；DONT后停止通知            |
| 其他选项   | WILL返回DONT，DO返回WONT；重复已达成协商不反复确认                                       |
| 子协商     | 跨块解析并剥离；最多4096字节，超限/非法IAC序列以TELNET_PROTOCOL_ERROR结束，不无限缓存    |
| NVT接收    | CR NUL显示为CR，CR LF保留，跨包状态保留；BINARY方向不抑制NUL                             |
| NVT发送    | xterm Enter的CR转换为CR LF，LF也转换为CR LF；跨输入块CR LF只发送一次，BINARY方向原样传递 |

初始化请求本地WILL BINARY/SGA/TTYPE/NAWS及远端DO BINARY/ECHO/SGA。每个方向分别维护No/WantYes/Yes，主动请求的确认不再回发正响应，拒绝后可接受服务端后续启用，不引入主动反复开关选项的第二套策略。所有协商响应及用户输入按同一有界队列发送，避免字节交错。

这些是基础交互终端所需子集，不实现LINEMODE、本地行编辑、环境变量协商、压缩、Telnet认证/加密扩展、URG/TCP同步或协议调试命令入口。某些旧设备若依赖上述能力或非UTF-8编码，需要后续扩展。

## 4. 生命周期与资源

TelnetFactory注册到现有SessionManager，每个连接独立。Connecting后TCP建立即Connected；登录提示作为普通输出，不生成SSH challenge。正常EOF为Closed，网络/协议错误在已连接后由core转为Disconnected，错误与原缓冲保留；重连在同terminalId/xterm内创建新后端session，历史不清空。

TCP读取块8192字节，socket写队列最多8项，单个输入受core的16384字节上限限制，IAC/NVT转义最多扩为32768字节。读取与写入future并发；写入单包最多等待30秒，输出等待共用core字节/消息预算和ACK信用。取消位于外层select，能打断建连、写入、等待队列或无ACK的输出，不派生遗留网络任务。TCP stream随驱动释放而关闭。

支持复制活动连接、普通Tab、嵌套Workspace、输入广播及尺寸变化；每个复制会话独立连接与手动登录。SFTP只接受SSH，Telnet不会注册SSH channel或运行主机系统探测。连接摘要使用telnet://host:port（IPv6带方括号），与SSH/Local/Serial共用最近20次历史，不保存正文或登录输入。

## 5. schema 2→3

Telnet 由 0002_telnet.sql 在初始父表约束中增加，迁移阶段结果为 schema 3。当前新库应用全部迁移到 schema 6，同加密格式 schema 2–5 解锁后原位升级，不迁移更早格式；数据库文件名、SQLCipher/主密钥/vault 元数据不变。

父表重建在专用初始化连接上进行，迁移事务外关闭外键，SQLx 迁移后恢复并检查 foreign_key_check，通过才发布业务池；保留已应用迁移原始字节与 SQLx 校验。完整版本表、故障与备份边界见 [存储升级](07-storage.md#6-schema-与迁移)。

## 6. 验证边界

已补回归源码但未编译执行：core地址/端口校验，协议任意分片/重复协商/拒绝/二进制与NVT/终端类型/NAWS转义/子协商限额，TCP本机临时端口夹具的输入/resize/EOF与无ACK取消，壳DTO的SSH字段拒绝，存储往返/混合字段拒绝/类型不可转换，以及加密schema 2升级后的主机、专属密码、分组标签、串口、历史关联和设置保留/重开。

历史交付仅执行类型、结构、格式、清单/锁、内存 SQLite 模型及文档差异静态检查，不等同 SQLCipher 或协议运行验收。

授权后验收应覆盖：

1. 三平台SQLCipher新库/旧schema 2升级、失败/中断后重试、引用与现有SSH/串口数据保留、拒绝早期库。
2. 常见Telnet服务及设备的23/自定义端口、IPv4/IPv6/DNS失败/超时、协商拒绝与未知选项、分片/恶意长协商。
3. 登录/密码回显、Enter/粘贴/Unicode/ANSI/全屏程序、NAWS同意/拒绝与窗口快速变化、BINARY方向独立切换。
4. 大量输出/慢消费者/无ACK/网络写入阻塞时关闭、远端EOF/复位、失败后复制与重连、稳定xterm缓冲与窗口关闭回收。
5. 保存/编辑/复制/配色失败重试、菜单与新标签页焦点交接、搜索/批量连接/分屏、SFTP隔离及三平台主题视觉。
