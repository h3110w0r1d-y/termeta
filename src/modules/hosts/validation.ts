import { t } from "@/i18n";
import type { FormItemRule } from "naive-ui";
import type { HostMetadata } from "@/modules/hosts/model";

// 与数据库的二进制名称唯一索引一致：去掉首尾空格，保留大小写。
export function uniqueHostNameRule(
  profiles: () => readonly Pick<HostMetadata, "id" | "name">[],
  exceptId: () => string | null | undefined,
): FormItemRule {
  return {
    trigger: ["input", "blur", "change"],
    validator: (_, value: unknown) =>
      typeof value !== "string" ||
      !profiles().some(
        (host) => host.id !== exceptId() && host.name === value.trim(),
      ) ||
      new Error(t("hosts.validation.nameConflict")),
  };
}
