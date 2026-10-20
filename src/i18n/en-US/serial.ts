// 使用稳定的语义 key，文案修改不改变调用标识。
export default {
  "serial.errors.missingConfiguration": "Serial configuration is missing",
  "serial.connect.title": "Serial connection",
  "serial.form.namePlaceholder": "e.g. Development board",
  "serial.form.stopBits": "Stop bits",
  "serial.parity.even": "Even",
  "serial.actions.refresh": "Refresh serial ports",
  "serial.parity.odd": "Odd",
  "serial.form.encoding": "Character set",
  "serial.form.tagsPlaceholder": "board, debugging",
  "serial.actions.open": "Opening serial port",
  "serial.form.dataBits": "Data bits",
  "serial.actions.newHost": "New serial host",
  "serial.empty":
    "No serial ports found. Connect a device and refresh, or enter its name manually. On Linux, also check serial port permissions.",
  "serial.form.parity": "Parity",
  "serial.form.baudRate": "Baud rate",
  "serial.validation.baudRate":
    "Baud rate must be a whole number from 1 to 4,000,000",
  "serial.form.flowControl": "Flow control",
  "serial.actions.editHost": "Edit serial host",
  "serial.validation.device":
    "Enter a serial device path or name, up to 1,024 bytes without control characters",
  "serial.form.devicePlaceholder":
    "Select or enter a serial device path or name",
  "serial.form.baudRatePlaceholder":
    "Select or enter a baud rate (default 9600)",
  "serial.actions.reopen":
    "Reopening serial port. Terminal history is preserved.",
  "serial.form.advanced": "Advanced settings",
} as const;
