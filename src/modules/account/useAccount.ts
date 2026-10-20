import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { t } from "@/i18n";
import { desktop, errorMessage } from "@/services/desktop/client";
import type {
  AccountOAuthFlow,
  AccountProvider,
  AccountProviderId,
  AccountStatus,
} from "@/services/desktop/types";

// WebView 仅持有公开摘要；凭据、浏览器地址和轮询 proof 始终留在 Rust。
export function useAccount(options: { loadProviders?: boolean } = {}) {
  const account = ref<AccountStatus>({ user: null, expiresAt: null });
  const providers = ref<AccountProvider[]>([
    { id: "termeta", name: "termeta.io" },
  ]);
  const flow = ref<AccountOAuthFlow | null>(null);
  const loading = ref(true);
  const working = ref(false);
  const error = ref("");
  const providerError = ref("");
  const busy = computed(() => loading.value || working.value);
  const blocked = computed(() => busy.value || flow.value !== null);
  let disposed = false;
  let generation = 0;
  let readGeneration = 0;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let deadlineTimer: ReturnType<typeof setTimeout> | undefined;
  let unlisten: (() => void) | undefined;

  function clearTimers() {
    clearTimeout(timer);
    clearTimeout(deadlineTimer);
  }
  async function readStatus(accountChanged = false) {
    const request = ++readGeneration;
    const status = await desktop.getAccountStatus();
    if (disposed || request !== readGeneration) return;
    const previousUserId = account.value.user?.id;
    account.value = status;
    // 账号失效通知立即终止旧等待；当前轮询成功登录产生的同账号通知不取消自己。
    if (
      accountChanged &&
      flow.value &&
      (!status.user || (previousUserId && previousUserId !== status.user.id))
    )
      await cancel();
    if (disposed || request !== readGeneration) return;
  }

  async function loadProviders() {
    providerError.value = "";
    try {
      const result = await desktop.listAccountProviders();
      if (!disposed)
        providers.value = [
          { id: "termeta", name: "termeta.io" },
          ...result.filter((provider) => provider.id !== "termeta"),
        ];
    } catch (cause) {
      if (!disposed) providerError.value = errorMessage(cause);
    }
  }
  async function perform(operation: () => Promise<unknown>) {
    if (disposed || blocked.value) return false;
    working.value = true;
    error.value = "";
    ++generation;
    ++readGeneration;
    try {
      await operation();
      if (!disposed) await readStatus();
      return !disposed;
    } catch (cause) {
      if (!disposed) {
        error.value = errorMessage(cause);
        // 退出或过期处理可能已清除本机凭据，即使远端请求失败也重新读取摘要。
        try {
          await readStatus();
        } catch {
          // 保留原操作的诊断，避免用后续读错误遮盖失败原因。
        }
      }
      return false;
    } finally {
      if (!disposed) working.value = false;
    }
  }
  async function cancel() {
    const pending = flow.value;
    if (!pending) return;
    ++generation;
    clearTimers();
    flow.value = null;
    working.value = true;
    try {
      await desktop.cancelAccountOAuth(pending.flowId);
    } catch (cause) {
      if (!disposed) error.value = errorMessage(cause);
    } finally {
      if (!disposed) working.value = false;
    }
  }
  async function poll(pending: AccountOAuthFlow, current: number) {
    if (disposed || current !== generation) return;
    try {
      const result = await desktop.pollAccountOAuth(pending.flowId);
      if (disposed || current !== generation) return;
      if (result.status === "pending") {
        timer = setTimeout(
          () => void poll(pending, current),
          Math.max(3, pending.pollInterval) * 1000,
        );
        return;
      }
      clearTimers();
      flow.value = null;
      if (result.status === "completed") {
        working.value = true;
        try {
          await readStatus();
        } finally {
          if (!disposed) working.value = false;
        }
      } else {
        error.value =
          result.error ||
          t(
            result.status === "expired"
              ? "account.errors.expired"
              : "account.errors.denied",
          );
      }
    } catch (cause) {
      if (disposed || current !== generation) return;
      clearTimers();
      flow.value = null;
      error.value = errorMessage(cause);
      // 单次兑换的响应丢失时不自动重试；清理 proof，允许用户重新发起。
      void desktop.cancelAccountOAuth(pending.flowId).catch(() => undefined);
    }
  }
  async function start(provider: AccountProviderId) {
    if (disposed || blocked.value) return;
    working.value = true;
    error.value = "";
    const current = ++generation;
    try {
      const pending = await desktop.startAccountOAuth(provider);
      if (disposed || current !== generation) {
        void desktop.cancelAccountOAuth(pending.flowId).catch(() => undefined);
        return;
      }
      const remaining = Date.parse(pending.expiresAt) - Date.now();
      if (!Number.isFinite(remaining) || remaining <= 0) {
        void desktop.cancelAccountOAuth(pending.flowId).catch(() => undefined);
        error.value = t("account.errors.expired");
        return;
      }
      flow.value = pending;
      deadlineTimer = setTimeout(
        () => {
          if (current !== generation) return;
          void cancel().then(() => {
            if (!disposed) {
              error.value = t("account.errors.expired");
            }
          });
        },
        Math.min(remaining, 600_000),
      );
      timer = setTimeout(
        () => void poll(pending, current),
        Math.max(3, pending.pollInterval) * 1000,
      );
    } catch (cause) {
      if (!disposed) error.value = errorMessage(cause);
    } finally {
      if (!disposed) working.value = false;
    }
  }
  onMounted(async () => {
    try {
      const stop = await desktop.onAccountChanged(() => {
        void readStatus(true).catch((cause) => {
          if (!disposed) error.value = errorMessage(cause);
        });
      });
      if (disposed) {
        stop();
        return;
      }
      unlisten = stop;
      await readStatus();
      if (options.loadProviders) await loadProviders();
    } catch (cause) {
      if (!disposed) error.value = errorMessage(cause);
    } finally {
      if (!disposed) loading.value = false;
    }
  });
  onBeforeUnmount(() => {
    disposed = true;
    ++generation;
    ++readGeneration;
    clearTimers();
    unlisten?.();
    const pending = flow.value;
    if (pending)
      void desktop.cancelAccountOAuth(pending.flowId).catch(() => undefined);
  });
  return {
    account,
    providers,
    flow,
    loading,
    working,
    busy,
    blocked,
    error,
    providerError,
    loadProviders,
    perform,
    start,
    cancel,
  };
}
