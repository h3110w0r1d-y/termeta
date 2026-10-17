import { t } from "../../i18n";
import type { SessionLogSummary } from "../../services/desktop/types";

export function sessionLogProtocol(protocol: SessionLogSummary["protocol"]) {
  if (protocol === "local") return t("common.connection.local");
  if (protocol === "serial") return t("common.connection.serial");
  if (protocol === "telnet") return "Telnet";
  return "SSH";
}

export function sessionLogEndpoint(log: SessionLogSummary) {
  const host = log.host ?? "";
  const endpoint = log.port
    ? `${host.includes(":") ? `[${host}]` : host}:${log.port}`
    : host;
  return log.username ? `${log.username}@${endpoint}` : endpoint;
}
