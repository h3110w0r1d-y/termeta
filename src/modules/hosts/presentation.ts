import type { HostProfile } from "../../services/desktop/types";

export function hostKindLabel(host: Pick<HostProfile, "kind">): string {
  return host.kind === "serial"
    ? "串口"
    : host.kind === "telnet"
      ? "Telnet"
      : "SSH";
}
export function hostAddress(host: HostProfile): string {
  return host.kind === "serial"
    ? (host.serialConfig?.path ?? "")
    : host.address;
}
export function hostEndpoint(host: HostProfile): string {
  if (host.kind === "serial") {
    const config = host.serialConfig;
    return config
      ? `${config.path} · ${config.baudRate} baud · ${config.charset}`
      : "串口配置缺失";
  }
  const address = host.address.includes(":")
    ? `[${host.address}]`
    : host.address;
  return host.kind === "telnet"
    ? `${address}:${host.port}`
    : `${host.username}@${address}:${host.port}`;
}
