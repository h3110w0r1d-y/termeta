# 测试入口与验证边界

Node 用例直接导入 TypeScript，要求 Node 22.18+。Rust 测试会编译；遵循 [协作规则](../AGENTS.md)，未经用户明确授权不执行项目测试、构建/启动应用或连接主机/设备。类型/格式/结构静态检查不替代运行验收。

## Node 用例

| 文件                                                       | 覆盖范围                                                            |
| ---------------------------------------------------------- | ------------------------------------------------------------------- |
| [workspace-layout.test.mjs](workspace-layout.test.mjs)     | 四向/嵌套分屏、迁移/收缩、比例/不变性、批量均分                     |
| [terminal-broadcast.test.mjs](terminal-broadcast.test.mjs) | 用户输入/设备应答/鼠标隔离、去重、光标模式、粘贴/滚轮递归保护与降级 |
| [host-search.test.mjs](host-search.test.mjs)               | 全拼/首字母/多音字/混输/多词、字段与地址匹配                        |
| [file-entries.test.mjs](file-entries.test.mjs)             | 目录/目录链接优先、排序/隐藏、分页不变性与断链展示                  |

以上用例仅有源码/语法静态检查记录，尚未执行。授权后的入口：

```sh
node --test tests/*.test.mjs
```

## Rust 回归源码

| 位置                                          | 范围                                                                                       |
| --------------------------------------------- | ------------------------------------------------------------------------------------------ |
| terminal-core/src/tests.rs                    | 会话 owner、配置/能力、流控/取消等核心行为                                                 |
| terminal-serial/src                           | 编码/字符边界/非法序列、设备路径与参数                                                     |
| terminal-telnet/src/protocol.rs、src/tests.rs | IAC/选项/NVT/NAWS/TTYPE/限额，以及本机 TCP 夹具 I/O/EOF/取消                               |
| terminal-ssh/src                              | 系统识别、转发规则/监听匹配、SOCKS5 分片/远端 DNS/拒绝 UDP 等                              |
| terminal-storage/src/tests.rs                 | 各类型持久化、凭据/分组事务、revision、工作区/名称唯一、转发失效引用、加密格式/迁移/原摘要 |
| terminal-security/src                         | 主密钥派生/校验、钥匙串注入/恢复、缓存失败与入口 future 的 Send 断言                       |
| src-tauri/src/commands、settings.rs           | DTO 字段隔离、设置白名单/旧数据兼容等                                                      |

历史串口授权调试中，macOS 的 terminal-core/terminal-serial 定向编译及15项测试通过；指定 ESP32-C3 输出/关闭和 xterm 无界面缓冲对比有局部证据，见 [串口记录](../docs/19-serial-terminal.md#5-已有验证与诊断入口)。这不覆盖后续新增 core/Telnet/存储集成；其他新增 Rust 回归源码尚未编译执行。

## 静态检查与运行验收

pnpm check:types 仅读取/检查前端类型，pnpm check:structure 仅读取源码/清单，pnpm check:format:vue 与 cargo fmt 仅格式检查，均不构建或启动项目。测试存在、格式解析、内存 SQLite 迁移模型不代表编译/SQLCipher 运行通过。

桌面/IME/焦点/剪贴板、标题栏/独立设置窗口、主机分组/搜索/批量连接、工作区/广播/存档、保险库迁移、SFTP/Finder 原生拖放、转发、实际 Telnet 和串口三平台矩阵，统一见 [运行验收边界](../docs/01-product-requirements.md#11-运行验收边界) 与各模块设计，避免重复维护多份清单。
