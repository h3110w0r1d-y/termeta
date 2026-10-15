import { defineStore } from "pinia";
import { ref } from "vue";
import { desktop } from "../../services/desktop/client";
import type {
  HostDraft,
  HostGroup,
  HostProfile,
} from "../../services/desktop/types";
export const useHosts = defineStore("hosts", () => {
  const hosts = ref<HostProfile[]>([]),
    groups = ref<HostGroup[]>([]);
  async function load() {
    [hosts.value, groups.value] = await Promise.all([
      desktop.listHosts(),
      desktop.listGroups(),
    ]);
  }
  async function save(host: HostDraft, revision: number | null) {
    const saved = await desktop.saveHost(host, revision);
    await load();
    return saved;
  }
  async function remove(host: HostProfile) {
    await desktop.deleteHost(host.id, host.revision);
    await load();
  }
  async function favorite(host: HostProfile) {
    const { revision, ...draft } = host;
    await save({ ...draft, favorite: !host.favorite }, revision);
  }
  return { hosts, groups, load, save, remove, favorite };
});
