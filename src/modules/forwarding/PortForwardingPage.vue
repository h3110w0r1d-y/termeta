<script setup lang="ts">
import { t } from "@/i18n";
import { computed, ref } from "vue";
import {
  NAlert,
  NButton,
  NCard,
  NEmpty,
  NFlex,
  NGrid,
  NGi,
  NH1,
  NInput,
  NTag,
  NText,
  useThemeVars,
} from "naive-ui";
import AssetIcon from "@/components/ui/AssetIcon.vue";
import { confirmAction } from "@/components/ui/confirmation";
import { errorMessage } from "@/services/desktop/client";
import type { ForwardProfile } from "@/services/desktop/types";
import { useHosts } from "../hosts/store";
import { stateLabels } from "../workbench/store";
import { usePortForwards } from "./store";
import ForwardEditor from "./ForwardEditor.vue";
const forwards = usePortForwards(),
  hosts = useHosts(),
  theme = useThemeVars();
const search = ref(""),
  error = ref(""),
  loading = ref(false),
  editorOpen = ref(false),
  editing = ref<ForwardProfile>(),
  copy = ref(false);
const modeLabels = computed(() => ({
  local: t("forwarding.modes.local"),
  remote: t("forwarding.modes.remote"),
  dynamic: t("forwarding.modes.dynamic"),
}));
const hostName = (profile: ForwardProfile) =>
  hosts.hosts.find((h) => h.id === profile.profileId)?.name;
const profiles = computed(() =>
  forwards.profiles.filter((profile) =>
    `${profile.name} ${hostName(profile) ?? ""} ${modeLabels.value[profile.rule.mode]} ${profile.rule.bindAddress} ${profile.rule.bindPort} ${profile.rule.targetAddress ?? ""} ${profile.rule.targetPort ?? ""}`
      .toLocaleLowerCase()
      .includes(search.value.trim().toLocaleLowerCase()),
  ),
);
function edit(profile?: ForwardProfile, duplicate = false) {
  editing.value = profile;
  copy.value = duplicate;
  editorOpen.value = true;
}
async function refresh() {
  if (loading.value) return;
  loading.value = true;
  error.value = "";
  try {
    await Promise.all([forwards.load(), hosts.load()]);
  } catch (cause) {
    error.value = errorMessage(cause);
  } finally {
    loading.value = false;
  }
}
async function remove(profile: ForwardProfile) {
  if (
    !(await confirmAction(
      t("forwarding.confirmDelete", {
        name: profile.name,
      }),
    ))
  )
    return;
  try {
    await forwards.remove(profile);
  } catch (cause) {
    error.value = errorMessage(cause);
  }
}
function endpoint(address: string, port: number | null) {
  return `${address.includes(":") ? `[${address}]` : address}:${port}`;
}
</script>
<template>
  <section class="forward-page">
    <NFlex align="center" justify="space-between">
      <NH1>{{ t("common.navigation.forwarding") }}</NH1>
      <NFlex>
        <NButton :loading="loading" @click="refresh">
          {{ t("common.actions.refresh") }}
        </NButton>
        <NButton type="primary" @click="edit()">
          <template #icon><AssetIcon name="add" /></template>
          {{ t("forwarding.actions.create") }}
        </NButton>
      </NFlex>
    </NFlex>
    <NInput
      v-model:value="search"
      clearable
      :placeholder="t('forwarding.search.placeholder')"
      :aria-label="t('forwarding.search.label')"
    />
    <NAlert v-if="error" type="error" closable @close="error = ''">
      {{ error }}
    </NAlert>
    <NEmpty
      v-if="!profiles.length"
      :description="
        search ? t('forwarding.search.noResults') : t('forwarding.empty')
      "
    />
    <NGrid
      v-else
      cols="1 720:2 1200:3"
      responsive="self"
      :x-gap="16"
      :y-gap="16"
    >
      <NGi v-for="profile in profiles" :key="profile.id">
        <NCard
          :title="profile.name"
          size="small"
          class="forward-card"
          :class="{
            running: forwards.runtimes[profile.id]?.state === 'connected',
          }"
          :style="{ '--forward-color': theme.primaryColor }"
          @dblclick="
            !forwards.active(profile.id) &&
            hostName(profile) &&
            forwards.start(profile.id)
          "
        >
          <template #header-extra>
            <NTag
              :type="
                forwards.runtimes[profile.id]?.state === 'connected'
                  ? 'primary'
                  : 'default'
              "
              size="small"
              :bordered="false"
            >
              {{
                forwards.runtimes[profile.id]
                  ? t(stateLabels[forwards.runtimes[profile.id]!.state])
                  : t("forwarding.status.stopped")
              }}
            </NTag>
          </template>
          <NFlex vertical :size="10">
            <NText depth="3">
              {{ modeLabels[profile.rule.mode] }} ·
              {{ hostName(profile) ?? t("forwarding.errors.hostDeleted") }}
            </NText>
            <NText class="endpoint">
              {{
                profile.rule.mode === "remote"
                  ? t("common.connection.remote")
                  : t("forwarding.endpoints.local")
              }}
              {{ endpoint(profile.rule.bindAddress, profile.rule.bindPort) }}
            </NText>
            <NText class="endpoint" depth="3">
              {{
                profile.rule.mode === "dynamic"
                  ? t("forwarding.endpoints.dynamicTarget")
                  : t("forwarding.endpoints.target", {
                      value1:
                        profile.rule.mode === "remote"
                          ? t("forwarding.endpoints.localSide")
                          : t("forwarding.endpoints.remoteSide"),
                      value2: endpoint(
                        profile.rule.targetAddress ?? "",
                        profile.rule.targetPort,
                      ),
                    })
              }}
            </NText>
            <NAlert
              v-if="forwards.runtimes[profile.id]?.error"
              type="error"
              :show-icon="false"
            >
              {{ forwards.runtimes[profile.id]!.error }}
            </NAlert>
          </NFlex>
          <template #action>
            <NFlex justify="space-between" @dblclick.stop>
              <NButton
                v-if="forwards.active(profile.id)"
                :loading="forwards.runtimes[profile.id]?.busy"
                :disabled="forwards.runtimes[profile.id]?.state === 'closing'"
                @click="forwards.stop(profile.id)"
              >
                {{ t("common.actions.stop") }}
              </NButton>
              <NButton
                v-else
                type="primary"
                :disabled="!hostName(profile)"
                @click="forwards.start(profile.id)"
              >
                {{ t("common.actions.start") }}
              </NButton>
              <NFlex :size="4">
                <NButton
                  size="small"
                  quaternary
                  :disabled="forwards.active(profile.id)"
                  @click="edit(profile)"
                >
                  {{ t("common.actions.edit") }}
                </NButton>
                <NButton size="small" quaternary @click="edit(profile, true)">
                  {{ t("common.actions.duplicate") }}
                </NButton>
                <NButton
                  size="small"
                  quaternary
                  :disabled="forwards.active(profile.id)"
                  @click="remove(profile)"
                >
                  {{ t("common.actions.delete") }}
                </NButton>
              </NFlex>
            </NFlex>
          </template>
        </NCard>
      </NGi>
    </NGrid>
    <ForwardEditor v-model:open="editorOpen" :profile="editing" :copy="copy" />
  </section>
</template>
<style scoped>
.forward-page {
  padding: 28px;
  display: flex;
  flex-direction: column;
  gap: 20px;
}
.forward-page :deep(h1) {
  margin: 0;
}
.forward-card {
  height: 100%;
}
.forward-card.running {
  border-color: var(--forward-color);
  background: color-mix(in srgb, var(--forward-color) 9%, var(--surface));
}
.endpoint {
  overflow-wrap: anywhere;
}
</style>
