import { computed, onBeforeUnmount, ref } from "vue";
import { desktop, errorMessage } from "@/services/desktop/client";
import type { FileEntry } from "@/services/desktop/types";

interface DirectoryOptions {
  sessionId: () => string | undefined;
  connected: () => boolean;
  busy: () => boolean;
  beforeLoad?: () => void;
}

/** 共享目录投影与读取代次；是否保留旧列表由所属浏览区的连接策略决定。 */
export function useSftpDirectory(options: DirectoryOptions) {
  const directory = ref(".");
  const entries = ref<FileEntry[]>([]);
  const selectedNames = ref<string[]>([]);
  const loading = ref(false);
  const error = ref("");
  const hasMore = ref(false);
  const listSession = ref<string>();
  const selectedEntries = computed(() => {
    const names = new Set(selectedNames.value);
    return entries.value.filter((entry) => names.has(entry.name));
  });
  const stale = computed(
    () => !options.connected() || listSession.value !== options.sessionId(),
  );
  let generation = 0;
  let disposed = false;

  async function load(path = directory.value, more = false) {
    const sessionId = options.sessionId();
    if (
      !sessionId ||
      !options.connected() ||
      loading.value ||
      options.busy() ||
      disposed
    )
      return;
    options.beforeLoad?.();
    const current = ++generation;
    loading.value = true;
    error.value = "";
    try {
      const page = await desktop.sftpList(sessionId, path, more);
      if (
        disposed ||
        current !== generation ||
        sessionId !== options.sessionId()
      )
        return;
      directory.value = page.path;
      entries.value = more ? [...entries.value, ...page.entries] : page.entries;
      hasMore.value = page.hasMore;
      listSession.value = sessionId;
      if (!more) selectedNames.value = [];
    } catch (cause) {
      if (!disposed && current === generation) {
        error.value = errorMessage(cause);
        listSession.value = undefined;
      }
    } finally {
      if (current === generation) loading.value = false;
    }
  }

  function sameSession(sessionId: string | undefined) {
    return (
      !disposed &&
      options.connected() &&
      sessionId === options.sessionId() &&
      listSession.value === sessionId
    );
  }
  function invalidate() {
    generation++;
    loading.value = false;
  }
  function reset() {
    invalidate();
    listSession.value = undefined;
    directory.value = ".";
    entries.value = [];
    selectedNames.value = [];
    hasMore.value = false;
  }
  onBeforeUnmount(() => {
    disposed = true;
    invalidate();
  });
  return {
    directory,
    entries,
    selectedNames,
    selectedEntries,
    loading,
    error,
    hasMore,
    stale,
    load,
    sameSession,
    invalidate,
    reset,
  };
}
