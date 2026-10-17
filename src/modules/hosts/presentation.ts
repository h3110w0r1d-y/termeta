import { t } from "../../i18n";
import type { HostProfile } from "../../services/desktop/types";

export function hostKindLabel(host: Pick<HostProfile, "kind">): string {
  return host.kind === "workspace"
    ? t("工作区")
    : host.kind === "serial"
      ? t("串口")
      : host.kind === "telnet"
        ? "Telnet"
        : "SSH";
}
export function hostAddress(host: HostProfile): string {
  return host.kind === "workspace"
    ? t("工作区")
    : host.kind === "serial"
      ? (host.serialConfig?.path ?? "")
      : host.address;
}
export function hostEndpoint(host: HostProfile): string {
  if (host.kind === "workspace") {
    const count = (
      node: import("../../services/desktop/types").SavedLayout,
    ): number =>
      node.kind === "connection" ? 1 : count(node.first) + count(node.second);
    return host.workspace
      ? t("{count} 个连接", { count: count(host.workspace.root) })
      : t("工作区布局缺失");
  }
  if (host.kind === "serial") {
    const config = host.serialConfig;
    return config
      ? `${config.path} · ${config.baudRate} baud · ${config.charset}`
      : t("串口配置缺失");
  }
  const address = host.address.includes(":")
    ? `[${host.address}]`
    : host.address;
  return host.kind === "telnet"
    ? `${address}:${host.port}`
    : `${host.username}@${address}:${host.port}`;
}
