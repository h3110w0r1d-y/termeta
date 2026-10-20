import { defineStore } from "pinia";
import { ref } from "vue";
import { desktop } from "@/services/desktop/client";
import type {
  HostDraft,
  HostGroup,
  HostProfile,
  WorkspaceDraft,
} from "@/services/desktop/types";
export const useHosts = defineStore("hosts", () => {
  const managementBusy = ref(false);
  const hosts = ref<HostProfile[]>([]),
    groups = ref<HostGroup[]>([]);
  let loadRevision = 0;
  async function load() {
    const revision = ++loadRevision;
    const inventory = await desktop.listHostInventory();
    // 主机与分组来自同一数据库快照；提交成功的写操作也会使更早的读取失效。
    if (revision !== loadRevision) return;
    hosts.value = inventory.hosts;
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
    hosts.value = [
      ...hosts.value.filter((item) => item.id !== saved.id),
      saved,
    ].sort(
      (a, b) =>
        Number(b.favorite) - Number(a.favorite) || a.name.localeCompare(b.name),
    );
    return saved;
  }
  async function saveWorkspace(draft: WorkspaceDraft) {
    const saved = await desktop.saveWorkspace(draft);
    loadRevision++;
    hosts.value = [
      ...hosts.value.filter((item) => item.id !== saved.id),
      saved,
    ].sort(
      (a, b) =>
        Number(b.favorite) - Number(a.favorite) || a.name.localeCompare(b.name),
    );
    return saved;
  }
  async function remove(host: HostProfile) {
    await desktop.deleteHost(host.id, host.revision);
    loadRevision++;
    // 删除已提交，直接更新列表，避免重读失败被误报为删除失败。
    hosts.value = hosts.value.filter((item) => item.id !== host.id);
  }
  async function favorite(host: HostProfile) {
    const { revision, hasPassword: _hasPassword, ...draft } = host;
    await save({ ...draft, favorite: !host.favorite }, revision);
  }
  async function moveHost(host: HostProfile, groupId: string | null) {
    if (host.groupId === groupId) return;
    const { revision, hasPassword: _hasPassword, ...draft } = host;
    // 不传密码字段，保持专属密码和公共认证引用，不读取或重写秘密。
    await save({ ...draft, groupId }, revision);
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
    groups,
    load,
    save,
    saveWorkspace,
    remove,
    favorite,
    moveHost,
    saveGroup,
  };
});
