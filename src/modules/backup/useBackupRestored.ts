import { onBeforeUnmount, onMounted } from "vue";
import { desktop } from "@/services/desktop/client";

// 备份只通知轻量数据发生变化，不在窗口事件中携带备份正文或凭据。
export function useBackupRestored(
  reload: () => Promise<void>,
  onError: (cause: unknown) => void,
) {
  let stop: (() => void) | undefined;
  let disposed = false;
  let pending = Promise.resolve();
  onMounted(async () => {
    try {
      const unlisten = await desktop.onBackupRestored(() => {
        pending = pending
          .then(() => {
            if (!disposed) return reload();
          })
          .catch(onError);
      });
      if (disposed) unlisten();
      else stop = unlisten;
    } catch (cause) {
      onError(cause);
    }
  });
  onBeforeUnmount(() => {
    disposed = true;
    stop?.();
  });
}
