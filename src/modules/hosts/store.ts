import { defineStore } from "pinia";
import { ref } from "vue";
import { desktop } from "../../services/desktop/client";
import type {
  HostDraft,
  HostGroup,
  HostProfile,
} from "../../services/desktop/types";
export const useHosts = defineStore("hosts", () => {
  const managementBusy = ref(false);
  const hosts = ref<HostProfile[]>([]),
    groups = ref<HostGroup[]>([]);
  async function load() {
    [hosts.value, groups.value] = await Promise.all([
      desktop.listHosts(),
      desktop.listGroups(),
    ]);
  }
  async function save(
    host: HostDraft,
    revision: number | null,
    password?: string,
  ) {
    const saved = await desktop.saveHost(host, revision, password);
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
  async function remove(host: HostProfile) {
    await desktop.deleteHost(host.id, host.revision);
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
    remove,
    favorite,
    moveHost,
    saveGroup,
  };
});
