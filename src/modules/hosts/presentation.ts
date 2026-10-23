import { t } from "@/i18n";
import type { HostProfile } from "@/modules/hosts/model";

export function hostKindLabel(host: Pick<HostProfile, "kind">): string {
  return host.kind === "workspace"
    ? t("workspaces.title")
    : host.kind === "serial"
      ? t("common.connection.serial")
      : host.kind === "telnet"
        ? "Telnet"
        : "SSH";
}
export function hostAddress(host: HostProfile): string {
  return host.kind === "workspace"
    ? t("workspaces.title")
    : host.kind === "serial"
      ? host.serialConfig.path
      : host.address;
}
export function hostEndpoint(host: HostProfile): string {
  if (host.kind === "workspace") {
    const count = (node: import("./model").SavedLayout): number =>
      node.kind === "connection" ? 1 : count(node.first) + count(node.second);
    return t("common.count.connections", { count: count(host.workspace.root) });
  }
  if (host.kind === "serial") {
    const config = host.serialConfig;
    return `${config.path} · ${config.baudRate} baud · ${config.charset}`;
  }
  const address = host.address.includes(":")
    ? `[${host.address}]`
    : host.address;
  return host.kind === "telnet"
    ? `${address}:${host.port}`
    : `${host.username}@${address}:${host.port}`;
}
