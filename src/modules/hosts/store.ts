import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { compareHostDisplayOrder, compareHostOrder } from "./ordering";
import { useSettings } from "../settings/store";
import { desktop } from "@/services/desktop/client";
import type {
  HostDraft,
  HostGroup,
  HostProfile,
  WorkspaceDraft,
} from "@/modules/hosts/model";
export const useHosts = defineStore("hosts", () => {
  const settings = useSettings();
  const managementBusy = ref(false);
  const hosts = ref<HostProfile[]>([]),
    groups = ref<HostGroup[]>([]);
  const manualOrder = computed(() => settings.value.hostSortMode === "manual");
  const orderedHosts = computed(() =>
    [...hosts.value].sort((a, b) =>
      compareHostDisplayOrder(a, b, settings.value.hostSortMode),
    ),
  );
  function acceptHosts(next: HostProfile[]) {
    const recent = new Map(
      hosts.value.map((host) => [host.id, host.lastUsedAt]),
    );
    // 读取与使用通知可以交错，迟到的列表或保存结果不能回退最近使用时间。
    hosts.value = next.map((host) => {
      const previous = recent.get(host.id);
      return previous != null && previous > (host.lastUsedAt ?? -1)
        ? { ...host, lastUsedAt: previous }
        : host;
    });
  }
  function markUsed(profileId: string, lastUsedAt: number) {
    const host = hosts.value.find((item) => item.id === profileId);
    if (host && lastUsedAt > (host.lastUsedAt ?? -1))
      host.lastUsedAt = lastUsedAt;
  }
  let loadRevision = 0;
  async function load() {
    const revision = ++loadRevision;
    const inventory = await desktop.listHostInventory();
    // 主机与分组来自同一数据库快照；提交成功的写操作也会使更早的读取失效。
    if (revision !== loadRevision) return;
    acceptHosts(inventory.hosts);
    groups.value = inventory.groups;
  }
  async function save(
    host: HostDraft,
    revision: number | null,
    password?: string,
  ) {
    const saved = await desktop.saveHost(host, revision, password);
    loadRevision++;
    // 提交成功后直接更新轻量列表，避免后续读取失败被误判为保存失败。
    acceptHosts(
      [...hosts.value.filter((item) => item.id !== saved.id), saved].sort(
        compareHostOrder,
      ),
    );
    return saved;
  }
  async function saveWorkspace(draft: WorkspaceDraft) {
    const saved = await desktop.saveWorkspace(draft);
    loadRevision++;
    acceptHosts(
      [...hosts.value.filter((item) => item.id !== saved.id), saved].sort(
        compareHostOrder,
      ),
    );
    return saved;
  }
  async function remove(host: HostProfile) {
    await desktop.deleteHost(host.id, host.revision);
    loadRevision++;
    // 删除已提交，直接更新列表，避免重读失败被误报为删除失败。
    hosts.value = hosts.value.filter((item) => item.id !== host.id);
  }
  async function arrangeHosts(
    orderedIds: string[],
    moveTo: { hostIds: string[]; groupId: string | null } | null = null,
    preserveOrder = false,
  ) {
    const byId = new Map(hosts.value.map((host) => [host.id, host]));
    const inventory = await desktop.arrangeHosts({
      orderedHosts: orderedIds.map((id) => ({
        id,
        revision: byId.get(id)?.revision ?? -1,
      })),
      moveTo,
      preserveOrder,
    });
    loadRevision++;
    acceptHosts(inventory.hosts);
    groups.value = inventory.groups;
  }
  async function saveGroup(
    group: Omit<HostGroup, "id" | "revision"> & { id: string | null },
    revision: number | null,
  ) {
    const saved = await desktop.saveGroup(group, revision);
    loadRevision++;
    groups.value = [
      ...groups.value.filter((item) => item.id !== saved.id),
      saved,
    ];
    return saved;
  }
  return {
    managementBusy,
    hosts,
    orderedHosts,
    manualOrder,
    markUsed,
    groups,
    load,
    save,
    saveWorkspace,
    remove,
    arrangeHosts,
    saveGroup,
  };
});
