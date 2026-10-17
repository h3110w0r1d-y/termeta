import { locale, t } from "../../i18n";
import type { Terminal } from "@xterm/xterm";

/** 在连接边界追加可信本地消息，不恢复旧光标或清空终端历史。 */
export function writeLocalStatus(term: Terminal, message: string) {
  const safe = (text: string) => text.replace(/[\x00-\x1f\x7f-\x9f]/g, " ");
  let snapshot = "";
  let leaveAlternate = "";
  if (term.buffer.active.type === "alternate") {
    // 仅退出实际激活的备用缓冲区，不恢复旧连接保存的光标。
    // 1047 仍会带入备用屏光标，因此退出后在普通屏底部追加快照，避免覆盖历史。
    leaveAlternate = `\x1b[?1047l\x1b[?6l\x1b[r\x1b[${term.rows};1H`;
    const buffer = term.buffer.active;
    const lines: string[] = [];
    let bytes = 0;
    for (let row = 0; row < Math.min(1000, term.rows); row++) {
      const line = safe(
        buffer.getLine(buffer.baseY + row)?.translateToString(true) ?? "",
      );
      bytes += new TextEncoder().encode(line).length;
      if (bytes > 1024 * 1024) {
        lines.push(t("terminal.snapshot.truncated"));
        break;
      }
      lines.push(line);
    }
    snapshot = t("terminal.snapshot.header") + lines.join("\r\n") + "\r\n";
  }
  // v6 Utf8ToUtf32 对 ASCII 非续字节会丢弃残留；必须用 bytes 写 CAN，字符串使用另一解码器。
  // 普通缓冲区不切屏、不恢复光标；全屏末帧有界保留后才退出备用缓冲区。
  const modes =
    "\x18\x1b[0m\x1b[?2004l\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1006l";
  term.write(
    new TextEncoder().encode(
      modes +
        leaveAlternate +
        snapshot +
        `\r\n\x1b[90m[${new Date().toLocaleTimeString(locale.value)}] ${safe(message)}\x1b[0m\r\n`,
    ),
  );
}
