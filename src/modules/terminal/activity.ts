import { normalizeRemoteTitle } from "./title.ts";
import type { TerminalWorkingDirectory } from "./workingDirectory";

/** 只保留当前连接的提示信息，不保存命令正文或执行历史。 */
export interface TerminalActivity extends TerminalWorkingDirectory {
  sshCommand?: boolean;
  remoteTitle?: string;
  initialTitlePrefix?: string;
  commandRunning?: boolean;
}

export function updateActivityTitle(activity: TerminalActivity, title: string) {
  const normalized = normalizeRemoteTitle(title);
  activity.initialTitlePrefix ??= titlePrefix(normalized);
  activity.remoteTitle = normalized || undefined;
}

/** 首个冒号之后通常是目录；目录变化不作为命令执行依据。 */
function titlePrefix(title: string): string {
  return title.split(":", 1)[0]!.trim();
}

export function updateShellActivity(
  activity: TerminalActivity,
  payload: string,
) {
  const marker = payload.split(";", 1)[0];
  if (marker === "C") activity.commandRunning = true;
  else if (marker === "A" || marker === "B" || marker === "D")
    activity.commandRunning = false;
}

export function resetTerminalActivity(activity: TerminalActivity) {
  activity.workingDirectory = undefined;
  activity.workingDirectorySource = undefined;
  activity.remoteTitle = undefined;
  activity.initialTitlePrefix = undefined;
  activity.commandRunning = false;
}

/** 命令模式始终视为运行中；交互 Shell 沿用 OSC 133 和标题提示。 */
export function needsCloseConfirmation(
  activity: TerminalActivity & { state: string },
): boolean {
  return (
    activity.state === "connected" &&
    (!!activity.sshCommand ||
      !!activity.commandRunning ||
      (activity.initialTitlePrefix !== undefined &&
        titlePrefix(activity.remoteTitle ?? "") !==
          activity.initialTitlePrefix))
  );
}
