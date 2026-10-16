import { computed, onMounted, onBeforeUnmount, ref, watch } from "vue";
import { darkTheme } from "naive-ui";
import { useSettings } from "../modules/settings/store";

export function useAppAppearance() {
  const settings = useSettings();
  const systemAppearance = window.matchMedia("(prefers-color-scheme: dark)");
  const systemDark = ref(systemAppearance.matches);
  const naiveTheme = computed(() =>
    settings.value.appearance === "dark" ||
    (settings.value.appearance === "system" && systemDark.value)
      ? darkTheme
      : null,
  );
  function updateSystemAppearance(event: MediaQueryListEvent) {
    systemDark.value = event.matches;
  }
  watch(
    () => settings.value.appearance,
    (value) => {
      document.documentElement.dataset.appearance = value;
    },
    { immediate: true },
  );
  onMounted(() =>
    systemAppearance.addEventListener("change", updateSystemAppearance),
  );
  onBeforeUnmount(() =>
    systemAppearance.removeEventListener("change", updateSystemAppearance),
  );
  return { naiveTheme };
}
