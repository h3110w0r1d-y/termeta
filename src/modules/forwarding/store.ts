import { t } from "@/i18n";
import { defineStore } from "pinia";
import { computed, onScopeDispose, reactive, ref, watch } from "vue";
import { desktop, errorMessage } from "@/services/desktop/client";
import type {
  ForwardDraft,
  ForwardProfile,
  SessionEvent,
  SessionSnapshot,
} from "@/services/desktop/types";
import { useSessions } from "../sessions/store";
import {
  createSessionControl,
  isSessionTerminal,
  type SessionControl,
} from "../sessions/control";

interface Runtime extends SessionControl {
  error: string;
  busy: boolean;
}
export const usePortForwards = defineStore("port-forwards", () => {
  const sessions = useSessions();
  const profiles = ref<ForwardProfile[]>([]);
  const runtimes = reactive<Record<string, Runtime>>({});
  const generations = new Map<string, number>();
  const active = (id: string) => {
    const runtime = runtimes[id];
    return !!runtime && (runtime.busy || !isSessionTerminal(runtime.state));
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
  const title = (id: string) =>
    profiles.value.find((profile) => profile.id === id)?.name ??
    t("common.navigation.forwarding");
  function update(id: string, snapshot: SessionSnapshot) {
    const runtime = runtimes[id];
    return runtime
      ? sessions.snapshot(runtime, snapshot, "forward", title(id))
      : false;
  }
  function control(id: string, generation: number, event: SessionEvent) {
    if (generations.get(id) !== generation) return;
    const runtime = runtimes[id];
    if (!runtime || !sessions.event(runtime, event, "forward", title(id)))
      return;
    if (event.type === "ended")
      runtime.error = event.error ? errorMessage(event.error) : "";
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
      const terminal = isSessionTerminal(snapshot.state);
      if (terminal && runtimes[id]?.sessionId !== snapshot.sessionId) continue;
      if (
        !runtimes[id] ||
        (runtimes[id]!.ended &&
          !terminal &&
          runtimes[id]!.sessionId !== snapshot.sessionId)
      )
        runtimes[id] = {
          ...createSessionControl(snapshot.requestId),
          error: "",
          busy: false,
        };
      if (!update(id, snapshot)) continue;
      if (terminal && runtimes[id]?.sessionId === snapshot.sessionId) {
        runtimes[id]!.ended = true;
        sessions.release("forward", snapshot.sessionId);
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
    const runtime = runtimes[profile.id];
    if (runtime?.sessionId) sessions.release("forward", runtime.sessionId);
    delete runtimes[profile.id];
    generations.delete(profile.id);
  }
  async function start(id: string) {
    if (active(id)) return;
    const generation = (generations.get(id) ?? 0) + 1;
    generations.set(id, generation);
    runtimes[id] = {
      ...createSessionControl(crypto.randomUUID()),
      error: "",
      busy: true,
    };
    const runtime = runtimes[id]!;
    try {
      update(
        id,
        await desktop.startPortForward(id, runtime.requestId, (event) =>
          control(id, generation, event),
        ),
      );
    } catch (cause) {
      // IPC响应丢失时重新读取后端，防止实际已启动却被界面遗忘。
      if (!runtime.ended) {
        runtime.state = "failed";
        runtime.error = errorMessage(cause);
      }
      try {
        await load();
      } catch {
        /* 原错误保留，可手动刷新 */
      }
    } finally {
      runtime.busy = false;
    }
  }
  async function stop(id: string) {
    const runtime = runtimes[id];
    if (!runtime || runtime.busy) return;
    runtime.busy = true;
    try {
      await desktop.stopPortForward(id);
      if (!runtime.ended) runtime.state = "closing";
      if (runtime.sessionId) {
        sessions.dismiss(runtime.sessionId);
      }
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
