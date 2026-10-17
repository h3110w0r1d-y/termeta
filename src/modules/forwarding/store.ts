import { t } from "../../i18n";
import { defineStore } from "pinia";
import { computed, onScopeDispose, reactive, ref, watch } from "vue";
import { desktop, errorMessage } from "../../services/desktop/client";
import type {
  ForwardDraft,
  ForwardProfile,
  SessionEvent,
  SessionSnapshot,
  SessionState,
} from "../../services/desktop/types";
import { useWorkbenchStore } from "../workbench/store";

interface Runtime {
  sessionId?: string;
  state: SessionState;
  revision: number;
  error: string;
  busy: boolean;
  ended: boolean;
}
export const usePortForwards = defineStore("port-forwards", () => {
  const workbench = useWorkbenchStore();
  const profiles = ref<ForwardProfile[]>([]);
  const runtimes = reactive<Record<string, Runtime>>({});
  const generations = new Map<string, number>();
  const active = (id: string) => {
    const runtime = runtimes[id];
    return (
      !!runtime &&
      (runtime.busy ||
        !["closed", "failed", "disconnected"].includes(runtime.state))
    );
  };
  const hasActive = computed(() => Object.keys(runtimes).some(active));
  // Channel负责即时状态；读取摘要补偿WebView恢复、丢失响应或控制接收端中断。
  let polling: ReturnType<typeof setInterval> | undefined;
  let pollingBusy = false;
  watch(hasActive, (running) => {
    if (polling) clearInterval(polling);
    polling = undefined;
    if (running)
      polling = setInterval(async () => {
        if (pollingBusy) return;
        pollingBusy = true;
        try {
          await load();
        } catch {
          /* 卡片保留当前状态，用户仍可停止或刷新 */
        } finally {
          pollingBusy = false;
        }
      }, 2000);
  });
  onScopeDispose(() => {
    if (polling) clearInterval(polling);
  });
  let loadGeneration = 0;
  function update(id: string, snapshot: SessionSnapshot) {
    const runtime = runtimes[id];
    if (
      !runtime ||
      runtime.ended ||
      (runtime.sessionId && runtime.sessionId !== snapshot.sessionId) ||
      snapshot.stateRevision <= runtime.revision
    )
      return;
    Object.assign(runtime, {
      sessionId: snapshot.sessionId,
      state: snapshot.state,
      revision: snapshot.stateRevision,
    });
    workbench.connectionTitles[snapshot.sessionId] =
      profiles.value.find((p) => p.id === id)?.name ?? snapshot.title;
  }
  function control(id: string, generation: number, event: SessionEvent) {
    if (generations.get(id) !== generation) return;
    const runtime = runtimes[id];
    if (!runtime || (runtime.ended && event.type !== "ended")) return;
    runtime.sessionId = event.sessionId;
    workbench.connectionTitles[event.sessionId] =
      profiles.value.find((p) => p.id === id)?.name ?? t("端口转发");
    if (event.type === "stateChanged") update(id, event.snapshot);
    else if (
      event.type === "hostKeyChallenge" ||
      event.type === "authChallenge"
    ) {
      workbench.dismiss(event.sessionId);
      workbench.challenges.push(event);
    } else if (event.type === "ended") {
      Object.assign(runtime, {
        state: event.finalState,
        ended: true,
        error: event.error?.message ?? "",
      });
      workbench.dismiss(event.sessionId);
      delete workbench.connectionTitles[event.sessionId];
    }
  }
  async function load() {
    const generation = ++loadGeneration;
    const attempts = new Map(generations);
    const result = await desktop.listPortForwards();
    if (generation !== loadGeneration) return;
    profiles.value = result.profiles;
    for (const snapshot of result.sessions) {
      const id = snapshot.terminalId;
      if ((generations.get(id) ?? 0) !== (attempts.get(id) ?? 0)) continue;
      const terminal = ["closed", "failed", "disconnected"].includes(
        snapshot.state,
      );
      if (terminal && runtimes[id]?.sessionId !== snapshot.sessionId) continue;
      if (
        !runtimes[id] ||
        (runtimes[id]!.ended &&
          !terminal &&
          runtimes[id]!.sessionId !== snapshot.sessionId)
      )
        runtimes[id] = {
          state: snapshot.state,
          revision: -1,
          error: "",
          busy: false,
          ended: false,
        };
      update(id, snapshot);
      if (terminal && runtimes[id]?.sessionId === snapshot.sessionId) {
        runtimes[id]!.ended = true;
        workbench.dismiss(snapshot.sessionId);
        delete workbench.connectionTitles[snapshot.sessionId];
      }
    }
  }
  async function save(draft: ForwardDraft, revision: number | null) {
    const saved = await desktop.savePortForward(draft, revision);
    ++loadGeneration;
    profiles.value = [
      ...profiles.value.filter((p) => p.id !== saved.id),
      saved,
    ].sort((a, b) => a.name.localeCompare(b.name));
  }
  async function remove(profile: ForwardProfile) {
    await desktop.deletePortForward(profile.id, profile.revision);
    ++loadGeneration;
    profiles.value = profiles.value.filter((p) => p.id !== profile.id);
    delete runtimes[profile.id];
    generations.delete(profile.id);
  }
  async function start(id: string) {
    if (active(id)) return;
    const generation = (generations.get(id) ?? 0) + 1;
    generations.set(id, generation);
    runtimes[id] = {
      state: "created",
      revision: -1,
      error: "",
      busy: true,
      ended: false,
    };
    const runtime = runtimes[id]!;
    try {
      update(
        id,
        await desktop.startPortForward(id, crypto.randomUUID(), (event) =>
          control(id, generation, event),
        ),
      );
    } catch (cause) {
      // IPC响应丢失时重新读取后端，防止实际已启动却被界面遗忘。
      runtime.state = "failed";
      runtime.error = errorMessage(cause);
      try {
        await load();
      } catch {
        /* 原错误保留，可手动刷新 */
      }
    } finally {
      runtimes[id]!.busy = false;
    }
  }
  async function stop(id: string) {
    const runtime = runtimes[id];
    if (!runtime || runtime.busy) return;
    runtime.busy = true;
    try {
      await desktop.stopPortForward(id);
      if (!runtime.ended) runtime.state = "closing";
      if (runtime.sessionId) workbench.dismiss(runtime.sessionId);
    } catch (cause) {
      runtime.error = errorMessage(cause);
    } finally {
      runtime.busy = false;
    }
  }
  return {
    profiles,
    runtimes,
    active,
    hasActive,
    load,
    save,
    remove,
    start,
    stop,
  };
});
