import type { FormItemRule } from "naive-ui";
import type { HostProfile } from "../../services/desktop/types";

// 与数据库的二进制名称唯一索引一致：去掉首尾空格，保留大小写。
export function uniqueHostNameRule(
  profiles: () => HostProfile[],
  exceptId: () => string | null | undefined,
): FormItemRule {
  return {
    trigger: ["input", "blur", "change"],
    validator: (_, value: unknown) =>
      typeof value !== "string" ||
      !profiles().some(
        (host) => host.id !== exceptId() && host.name === value.trim(),
      ) ||
      new Error("名称已存在，SSH、Telnet、串口与工作区名称不能重复"),
  };
}
