import assert from "node:assert/strict";
import test from "node:test";
import {
  needsCloseConfirmation,
  resetTerminalActivity,
  updateActivityTitle,
  updateShellActivity,
} from "../src/modules/terminal/activity.ts";

test("命令模式无标题也提醒，远端 Shell 标记不能解除实际命令状态", () => {
  const activity = { state: "connected", sshCommand: true };
  assert.equal(needsCloseConfirmation(activity), true);
  for (const marker of ["A", "B", "D;0"]) {
    updateShellActivity(activity, marker);
    updateActivityTitle(activity, "user@host:/tmp");
    assert.equal(needsCloseConfirmation(activity), true);
  }
  resetTerminalActivity(activity);
  assert.equal(needsCloseConfirmation(activity), true);
  for (const state of [
    "created",
    "opening",
    "closing",
    "closed",
    "failed",
    "disconnected",
  ]) {
    activity.state = state;
    assert.equal(needsCloseConfirmation(activity), false);
  }
});

test("新连接切回 Shell 后恢复原有关闭判断，不继承上次执行提示", () => {
  const activity = {
    state: "connected",
    sshCommand: true,
    workingDirectory: "/old",
    workingDirectorySource: "osc",
  };
  updateShellActivity(activity, "C");
  resetTerminalActivity(activity);
  assert.equal(activity.workingDirectory, undefined);
  assert.equal(activity.workingDirectorySource, undefined);
  activity.sshCommand = false;
  assert.equal(needsCloseConfirmation(activity), false);
  updateActivityTitle(activity, "user@host:~");
  updateActivityTitle(activity, "user@host:/var/log");
  assert.equal(needsCloseConfirmation(activity), false);
  updateShellActivity(activity, "C");
  assert.equal(needsCloseConfirmation(activity), true);
  updateShellActivity(activity, "D;0");
  assert.equal(needsCloseConfirmation(activity), false);
});
