import { onBeforeUnmount, onMounted, ref } from "vue";
import { desktop, errorMessage } from "@/services/desktop/client";
import type { OfficialSyncStatus } from "@/services/desktop/types";

// 窗口只订阅失效通知，重新通过 IPC 获取状态，不接收保险库正文。
export function useOfficialSync() {
  const sync = ref<OfficialSyncStatus>();
  const error = ref("");
  let disposed = false;
  let generation = 0;
  let pending: Promise<void> | undefined;
  let refreshRequested = false;
  let stop: (() => void) | undefined;
  async function readStatus() {
    do {
      refreshRequested = false;
      const current = ++generation;
      try {
        const result = await desktop.getOfficialSyncStatus();
        if (!disposed && current === generation && !refreshRequested) {
          sync.value = result;
          error.value = "";
        }
      } catch (cause) {
        if (!disposed && current === generation && !refreshRequested)
          error.value = errorMessage(cause);
      }
    } while (!disposed && refreshRequested);
  }
  function refresh(): Promise<void> {
    if (disposed) return Promise.resolve();
    // 同一窗口的并发读取共用结果，避免较快的旧摘要覆盖正在恢复的状态。
    pending ??= readStatus().finally(() => {
      pending = undefined;
      if (!disposed && refreshRequested) return refresh();
    });
    return pending;
  }
  function invalidate() {
    ++generation;
    sync.value = undefined;
    error.value = "";
    if (pending) refreshRequested = true;
  }
  onMounted(async () => {
    try {
      const unlisten = await desktop.onOfficialSyncChanged(() => {
        // 请求期间发生的变更必须在其完成后重读，不能被合并读取吞掉。
        if (pending) refreshRequested = true;
        void refresh();
      });
      if (disposed) unlisten();
      else stop = unlisten;
    } catch (cause) {
      if (!disposed) error.value = errorMessage(cause);
    }
  });
  onBeforeUnmount(() => {
    disposed = true;
    ++generation;
    stop?.();
  });
  return { sync, error, refresh, invalidate };
}
