// 字节数在IPC中为十进制字符串，使用整数换算避免大文件超出JS安全整数范围。
export function formatFileSize(value: string | null): string {
  if (value === null || !/^\d+$/.test(value)) return "—";
  const bytes = BigInt(value);
  const units = ["B", "KB", "MB", "GB", "TB"];
  let unit = 0;
  let divisor = 1n;
  while (unit < units.length - 1 && bytes >= divisor * 1024n) {
    divisor *= 1024n;
    unit++;
  }
  const hundredths = (bytes * 100n + divisor / 2n) / divisor;
  return `${hundredths / 100n}.${(hundredths % 100n).toString().padStart(2, "0")} ${units[unit]}`;
}
