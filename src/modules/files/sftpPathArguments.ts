const unsafeControls = /[\x00-\x1f\x7f-\x9f]/u;

/** 路径作为 POSIX Shell 的独立参数；单引号也能阻止交互 Shell 的感叹号展开。 */
function quotePath(path: string): string {
  return /^[a-zA-Z0-9_./-]+$/u.test(path)
    ? path
    : `'${path.replace(/'/gu, `'"'"'`)}'`;
}

/** 整批校验后才交给粘贴入口，拒绝控制字符，绝不生成换行或部分参数。 */
export function sftpPathArguments(
  directory: string,
  names: readonly string[],
): string | undefined {
  if (
    !directory.startsWith("/") ||
    unsafeControls.test(directory) ||
    directory.includes("\\") ||
    !names.length
  )
    return;
  const result: string[] = [];
  const encoder = new TextEncoder();
  let bytes = 0;
  for (const name of names) {
    if (
      !name ||
      name === "." ||
      name === ".." ||
      /[/\\]/u.test(name) ||
      unsafeControls.test(name)
    )
      return;
    const path = `${directory.replace(/\/+$/u, "")}/${name}`;
    const argument = quotePath(path);
    bytes += encoder.encode(argument).length + (result.length ? 1 : 0);
    if (bytes > 65536) return;
    result.push(argument);
  }
  return result.join(" ");
}
