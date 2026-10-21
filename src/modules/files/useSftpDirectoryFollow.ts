import { ref, watch } from "vue";

interface FollowOptions {
  enabled: () => boolean;
  sessionId: () => string | undefined;
  path: () => string | undefined;
  available: () => boolean;
  navigate: (path: string) => Promise<void>;
}

/** 忙碌或隐藏时只保留最新目录；失败不自动重试，取消勾选即丢弃待跳转。 */
export function useSftpDirectoryFollow(options: FollowOptions) {
  const pending = ref<{ sessionId: string; path: string }>();
  watch(
    () => [options.enabled(), options.sessionId(), options.path()] as const,
    ([enabled, sessionId, path]) => {
      pending.value =
        enabled && sessionId && path ? { sessionId, path } : undefined;
    },
    { immediate: true },
  );
  watch(
    () => [pending.value, options.available()] as const,
    ([target, available], _, onCleanup) => {
      if (!target || !available) return;
      // 合并一轮提示符内的连续上报，不打断正在进行的目录/文件操作。
      const timer = setTimeout(() => {
        if (
          !options.enabled() ||
          !options.available() ||
          target.sessionId !== options.sessionId() ||
          target.path !== options.path()
        )
          return;
        pending.value = undefined;
        void options.navigate(target.path);
      }, 120);
      onCleanup(() => clearTimeout(timer));
    },
    { immediate: true, flush: "post" },
  );
}
