# 本机 Telnet 测试环境

使用官方 Debian bookworm-slim 镜像及发行版 GNU inetutils telnetd/inetd 与 login；容器只映射到本机回环地址，无宿主目录挂载。这里的固定账号仅用于可丢弃的本机测试环境。

在仓库根目录启动：

```sh
docker compose -f dev/telnet/compose.yaml up -d --build
```

在 Termvexa 的“快速连接 → Telnet”或“新建主机 → Telnet”中填写：

| 字段 | 值 |
| --- | --- |
| 地址 | `127.0.0.1` |
| 端口 | `2323` |
| 登录用户名 | `termvexa` |
| 登录密码 | 启动后用 `docker compose exec telnet passwd termvexa` 设置 |

连接后在终端内输入账号和密码，密码输入时不显示字符。容器内root账号锁定，测试用户无sudo权限。Telnet为明文协议，此配置只绑定本机，不要改为对外开放固定测试账号。

登录横幅包含ANSI颜色及UTF-8文本。可用`stty size`检查窗口尺寸，拖动窗口/分屏后再次检查；`top`和`vim`检查全屏程序；`exit`断开后在原Tab重连检查历史保留。GNU inetutils telnetd用于基础交互验证，不能代表其他Telnet服务的全部协商行为已验收。

查看状态、停止、重新启动或移除容器：

```sh
docker compose -f dev/telnet/compose.yaml ps
docker compose -f dev/telnet/compose.yaml stop
docker compose -f dev/telnet/compose.yaml start
docker compose -f dev/telnet/compose.yaml down
```

容器中测试文件在删除容器后丢弃；停止容器会保留文件。不构建或启动Termvexa应用。
