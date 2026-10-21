import { t } from "@/i18n";
import { ref, watch } from "vue";
import type { FormInst } from "naive-ui";
import { clone } from "@/services/data/clone";
import { errorMessage } from "@/services/desktop/client";
import { validateBindings } from "@/services/commands/registry";
import { hostSchemePatch, useSettings } from "./store";
import type { AppSettings, SettingsPatch } from "./model";

export function useLiveSettings(form: { value: FormInst | undefined }) {
  const store = useSettings();
  const draft = ref(clone(store.value));
  let baseline = clone(store.value);
  const error = ref("");
  const busy = ref(false);
  let pending = Promise.resolve();
  function changed() {
    return (Object.keys(draft.value) as (keyof AppSettings)[]).filter(
      (key) =>
        JSON.stringify(draft.value[key]) !== JSON.stringify(baseline[key]),
    );
  }
  watch(
    () => store.value,
    (next) => {
      const dirty = new Set(changed());
      const merged = clone(draft.value);
      for (const key of Object.keys(next) as (keyof AppSettings)[])
        if (!dirty.has(key)) Object.assign(merged, { [key]: clone(next[key]) });
      if (dirty.has("hostSchemes")) {
        const local = hostSchemePatch(
          draft.value.hostSchemes,
          baseline.hostSchemes,
        );
        merged.hostSchemes = clone(next.hostSchemes);
        for (const [id, scheme] of Object.entries(local)) {
          if (scheme === null) delete merged.hostSchemes[id];
          else merged.hostSchemes[id] = scheme;
        }
      }
      baseline = clone(next);
      if (JSON.stringify(merged) !== JSON.stringify(draft.value))
        draft.value = merged;
    },
  );
  async function apply() {
    const keys = changed();
    if (!keys.length) return;
    const snapshot = clone(draft.value);
    try {
      if (form.value) await form.value.validate();
      validateBindings(snapshot.keymap.bindings);
      if (
        !Number.isFinite(snapshot.fontSize) ||
        snapshot.fontSize < 10 ||
        snapshot.fontSize > 32 ||
        !Number.isInteger(snapshot.scrollback) ||
        snapshot.scrollback < 1000 ||
        snapshot.scrollback > 50000
      )
        throw new Error(t("settings.validation.terminalRange"));
      busy.value = true;
      // 只提交本次修改的顶层字段，搜索和分组等即时偏好由各自入口管理。
      const patch: SettingsPatch = {};
      for (const key of keys)
        Object.assign(patch, { [key]: clone(snapshot[key]) });
      if (keys.includes("hostSchemes"))
        patch.hostSchemes = hostSchemePatch(
          snapshot.hostSchemes,
          baseline.hostSchemes,
        );
      await store.patch(patch);
      for (const key of keys)
        Object.assign(baseline, {
          [key]: clone(
            key === "hostSchemes" ? store.value.hostSchemes : snapshot[key],
          ),
        });
      error.value = "";
    } catch (e) {
      error.value = errorMessage(e) || t("settings.validation.invalid");
    } finally {
      busy.value = false;
    }
  }
  function enqueue() {
    pending = pending.then(apply);
    return pending;
  }
  watch(
    draft,
    () => {
      void enqueue();
    },
    { deep: true },
  );
  async function flush() {
    await enqueue();
    return !error.value && !changed().length;
  }
  return { draft, error, busy, flush, retry: enqueue };
}
