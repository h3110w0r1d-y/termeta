// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "serial.errors.missingConfiguration": "串口配置缺失",
  "serial.connect.title": "串口连接",
  "serial.form.namePlaceholder": "例如：开发板串口",
  "serial.form.stopBits": "停止位",
  "serial.parity.even": "偶校验",
  "serial.actions.refresh": "刷新串口",
  "serial.parity.odd": "奇校验",
  "serial.form.encoding": "字符集",
  "serial.form.tagsPlaceholder": "开发板, 调试",
  "serial.actions.open": "打开串口",
  "serial.form.dataBits": "数据位",
  "serial.actions.newHost": "新建串口主机",
  "serial.empty":
    "未发现串口。请连接设备后刷新，或手动输入设备名称。Linux 用户还需检查串口访问权限。",
  "serial.form.parity": "校验",
  "serial.form.baudRate": "波特率",
  "serial.validation.baudRate": "波特率须为 1–4000000 的整数",
  "serial.form.flowControl": "流控",
  "serial.actions.editHost": "编辑串口主机",
  "serial.validation.device":
    "请输入串口设备路径或名称，最长1024字节且不能含控制字符",
  "serial.form.devicePlaceholder": "选择或输入串口设备路径/名称",
  "serial.form.baudRatePlaceholder": "选择或输入波特率（默认9600）",
  "serial.actions.reopen": "重新打开串口，保留终端历史",
  "serial.form.advanced": "高级设置",
} as const;
