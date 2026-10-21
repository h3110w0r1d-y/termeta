export interface TerminalWorkingDirectory {
  workingDirectory?: string;
  workingDirectorySource?: "osc" | "title";
}

function validPath(path: string): boolean {
  return (
    new TextEncoder().encode(path).length <= 4096 &&
    !/[\x00-\x1f\x7f-\x9f\\]/u.test(path) &&
    (path.startsWith("/") || path === "~" || path.startsWith("~/"))
  );
}

/** 目录上报只用于同会话 SFTP 浏览，不打开 URI 或生成终端输入。 */
export function parseWorkingDirectory(
  payload: string,
  format: "osc7" | "osc1337" | "title",
): string | undefined {
  if (payload.length > 16384 || /[\x00-\x1f\x7f-\x9f\\]/u.test(payload)) return;
  let path: string;
  if (format === "osc7") {
    if (!/^file:\/\//i.test(payload)) return;
    try {
      const uri = new URL(payload);
      if (uri.username || uri.password || uri.port || uri.search || uri.hash)
        return;
      path = decodeURIComponent(uri.pathname);
    } catch {
      return;
    }
  } else if (format === "osc1337") {
    if (!payload.startsWith("CurrentDir=")) return;
    path = payload.slice("CurrentDir=".length);
  } else {
    // 只兼容完整的 user@host:目录 标题，不猜命令、提示符或截断路径。
    const match = /^[^@\s:]+@[^:\s]+:((?:\/|~(?:\/|$)).*)$/u.exec(payload);
    if (!match) return;
    path = match[1]!;
    if (/(?:^|\/)(?:\.{3}|…)(?=\/|$)/u.test(path)) return;
  }
  return validPath(path) ? path : undefined;
}

export function updateWorkingDirectory(
  terminal: TerminalWorkingDirectory,
  payload: string,
  format: "osc7" | "osc1337" | "title",
) {
  if (format === "title" && terminal.workingDirectorySource === "osc") return;
  const path = parseWorkingDirectory(payload, format);
  if (path === undefined) return;
  terminal.workingDirectory = path;
  terminal.workingDirectorySource = format === "title" ? "title" : "osc";
}

/** SFTP 相对路径以远端主目录为基准，不把波浪号作为字面目录名。 */
export function sftpWorkingDirectory(path: string): string {
  return path === "~"
    ? "."
    : path.startsWith("~/")
      ? `./${path.slice(2)}`
      : path;
}
