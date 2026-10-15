import { normalizeRemoteTitle } from "./title";

/** 只保留当前连接的提示信息，不保存命令正文或执行历史。 */
export interface TerminalActivity {
  remoteTitle?: string;
  initialRemoteTitle?: string;
  commandRunning?: boolean;
}

export function updateActivityTitle(activity: TerminalActivity, title: string) {
  const normalized = normalizeRemoteTitle(title);
  // 空标题也算首次标题，不能把后续标题变化误当作基准初始化。
  activity.initialRemoteTitle ??= normalized;
  activity.remoteTitle = normalized || undefined;
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
  activity.remoteTitle = undefined;
  activity.initialRemoteTitle = undefined;
  activity.commandRunning = false;
}

/** OSC 133执行中优先；否则仍按用户规则回退到首次标题比较。 */
export function needsCloseConfirmation(
  activity: TerminalActivity & { state: string },
): boolean {
  if (activity.state !== "connected") return false;
  return (
    !!activity.commandRunning ||
    (activity.initialRemoteTitle !== undefined &&
      (activity.remoteTitle ?? "") !== activity.initialRemoteTitle)
  );
}
