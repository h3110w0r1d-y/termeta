import { t, type PlainMessageKey } from "@/i18n";
import { computed, ref, type Ref } from "vue";
import { desktop, errorMessage } from "@/services/desktop/client";
import type { TransferProgress } from "@/services/desktop/types";
import type { TerminalItem } from "../workbench/store";

export function useSftpTransferQueue(options: {
  terminal: () => TerminalItem;
  busy: () => boolean;
  disposed: () => boolean;
  connected: () => boolean;
  sameSession: (sessionId: string | undefined) => boolean;
  refresh: (path?: string) => Promise<void>;
  error: Ref<string>;
}) {
  const { error } = options;
  const transferring = ref(false);
  const progress = ref<TransferProgress>();
  let currentTransfer: { sessionId: string; id: string } | undefined;
  type TransferTask = {
    id: string;
    sessionId: string;
    path: string;
    upload: boolean;
    name: string | null;
    dropId?: string;
    dragNames?: string[];
    dragDirectories?: string[];
    started?: boolean;
    status: "queued" | "running" | "done" | "cancelled" | "failed";
    progress?: TransferProgress;
    message?: string;
    messageKey?: PlainMessageKey;
    cancelRequested?: boolean;
  };
  const tasks = ref<TransferTask[]>([]);
  // 拖动手势不是传输；外部目标请求内容后才显示任务和下载进度。
  const visibleTasks = computed(() =>
    tasks.value.filter((task) => !task.dragNames || task.started),
  );
  const visibleTransfer = computed(
    () =>
      transferring.value &&
      visibleTasks.value.some((task) => task.status === "running"),
  );
  function enqueue(task: Omit<TransferTask, "id" | "status">) {
    // 原生拖动依赖当前鼠标手势，绝不排队到鼠标释放后再启动。
    if (
      task.dragNames &&
      (options.busy() ||
        tasks.value.some(
          (item) => item.status === "queued" || item.status === "running",
        ))
    ) {
      error.value = t("files.errors.dropBusy");
      return;
    }
    // 保留最近的任务记录，等待任务上限防止长期积压授权引用。
    if (tasks.value.filter((item) => item.status === "queued").length >= 32) {
      error.value = t("files.transfer.queueFull");
      return;
    }
    tasks.value = [
      ...tasks.value
        .filter((item) => item.status === "queued" || item.status === "running")
        .concat(
          tasks.value
            .filter(
              (item) => item.status !== "queued" && item.status !== "running",
            )
            .slice(-19),
        ),
      { ...task, id: crypto.randomUUID(), status: "queued" },
    ];
    if (!task.dragNames) options.terminal().sftpTransferring = true;
    void runQueue();
  }
  async function runQueue() {
    if (transferring.value || options.disposed()) return;
    const task = tasks.value.find((item) => item.status === "queued");
    if (!task) {
      options.terminal().sftpTransferring = false;
      return;
    }
    if (!options.sameSession(task.sessionId)) {
      task.status = "cancelled";
      task.messageKey = "files.transfer.connectionChanged";
      void runQueue();
      return;
    }
    task.status = "running";
    transferring.value = true;
    currentTransfer = { sessionId: task.sessionId, id: task.id };
    progress.value = undefined;
    const onProgress = (value: TransferProgress) => {
      if (currentTransfer?.id === task.id) {
        task.started = true;
        options.terminal().sftpTransferring = true;
        task.progress = value;
        progress.value = value;
        // 若取消早于后端任务登记，首次进度到达后补发取消。
        if (task.cancelRequested) {
          task.cancelRequested = false;
          void desktop.sftpCancel(task.sessionId, task.id).catch(() => {});
        }
      }
    };
    try {
      const saved = task.dragNames
        ? await desktop.sftpStartDrag(
            task.sessionId,
            task.id,
            task.path,
            task.dragNames,
            task.dragDirectories ?? [],
            onProgress,
          )
        : task.dropId
          ? await desktop.sftpDropUpload(
              task.sessionId,
              task.id,
              task.dropId,
              task.path,
              onProgress,
            )
          : await desktop.sftpTransfer(
              task.sessionId,
              task.id,
              task.path,
              task.name,
              task.upload,
              onProgress,
            );
      task.status = saved ? "done" : "cancelled";
      task.messageKey = saved
        ? "files.transfer.complete"
        : "common.status.cancelled";
    } catch (cause) {
      const cancelled =
        typeof cause === "object" &&
        cause !== null &&
        "code" in cause &&
        cause.code === "CANCELLED";
      task.status = cancelled ? "cancelled" : "failed";
      if (cancelled) task.messageKey = "common.status.cancelled";
      else task.message = errorMessage(cause);
      if (task.dragNames && !task.started && !cancelled)
        error.value = task.message ?? "";
    } finally {
      task.cancelRequested = false;
      transferring.value = false;
      currentTransfer = undefined;
    }
    if (task.dragNames && !task.started) {
      tasks.value = tasks.value.filter((item) => item.id !== task.id);
    } else if (options.sameSession(task.sessionId)) await options.refresh();
    else if (!options.disposed() && options.connected())
      await options.refresh(".");
    if (options.disposed()) return;
    void runQueue();
  }
  function cancelQueued(task: TransferTask) {
    task.status = "cancelled";
    task.messageKey = "files.transfer.removed";
    if (
      !transferring.value &&
      !tasks.value.some((item) => item.status === "queued")
    )
      options.terminal().sftpTransferring = false;
  }
  async function cancel() {
    const transfer = currentTransfer;
    if (!transfer) return;
    const task = tasks.value.find((item) => item.id === transfer.id);
    if (task) task.cancelRequested = true;
    try {
      await desktop.sftpCancel(transfer.sessionId, transfer.id);
    } catch (cause) {
      error.value = errorMessage(cause);
    }
  }
  function cancelActive() {
    if (currentTransfer)
      void desktop
        .sftpCancel(currentTransfer.sessionId, currentTransfer.id)
        .catch(() => {});
  }
  return {
    tasks,
    visibleTasks,
    visibleTransfer,
    transferring,
    progress,
    enqueue,
    cancelQueued,
    cancel,
    cancelActive,
  };
}
