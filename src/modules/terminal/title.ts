import { t } from "../../i18n";

/** 远端标题仅作受限展示文本；不修改原始终端字节或保存的连接名称。 */
export function normalizeRemoteTitle(title: string): string {
  return Array.from(
    title
      .replace(
        /[\x00-\x1f\x7f-\x9f\u2028\u2029\u202a-\u202e\u2066-\u2069]/g,
        " ",
      )
      .trim(),
  )
    .slice(0, 160)
    .join("");
}

export function terminalDisplayTitle(
  base: string,
  remoteTitle?: string,
): string {
  return remoteTitle ? `${base} (${remoteTitle})` : base;
}

/** 仅本地终端的内置名称参与翻译，用户名称和远端标题保持原样。 */
export function terminalBaseTitle(terminal: {
  title: string;
  baseTitle: string;
  target: { source: string };
}): string {
  return terminal.target.source === "local" && terminal.baseTitle === "本地终端"
    ? terminal.title.replace(
        /^本地终端(?= \(\d+\)$|$)/u,
        t("terminal.titles.local"),
      )
    : terminal.title;
}
