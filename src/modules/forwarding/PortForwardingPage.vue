<script setup lang="ts">
import { t } from "../../i18n";
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
import AssetIcon from "../../components/ui/AssetIcon.vue";
import { confirmAction } from "../../components/ui/confirmation";
import { errorMessage } from "../../services/desktop/client";
import type { ForwardProfile } from "../../services/desktop/types";
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
  local: t("本地转发"),
  remote: t("远程转发"),
  dynamic: t("动态 SOCKS5"),
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
      t("删除端口转发“{name}”？引用的 SSH 主机将保留。", {
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
      <NH1>{{ t("端口转发") }}</NH1>
      <NFlex>
        <NButton :loading="loading" @click="refresh">{{ t("刷新") }}</NButton>
        <NButton type="primary" @click="edit()">
          <template #icon><AssetIcon name="add" /></template>
          {{ t("新建转发") }}
        </NButton>
      </NFlex>
    </NFlex>
    <NInput
      v-model:value="search"
      clearable
      :placeholder="t('搜索名称、SSH 主机或转发地址')"
      :aria-label="t('搜索端口转发')"
    />
    <NAlert v-if="error" type="error" closable @close="error = ''">
      {{ error }}
    </NAlert>
    <NEmpty
      v-if="!profiles.length"
      :description="search ? t('没有匹配的端口转发') : t('还没有端口转发')"
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
                  : t("未启动")
              }}
            </NTag>
          </template>
          <NFlex vertical :size="10">
            <NText depth="3">
              {{ modeLabels[profile.rule.mode] }} ·
              {{ hostName(profile) ?? t("引用的 SSH 主机已删除") }}
            </NText>
            <NText class="endpoint">
              {{ profile.rule.mode === "remote" ? t("远程") : t("本机") }}
              {{ endpoint(profile.rule.bindAddress, profile.rule.bindPort) }}
            </NText>
            <NText class="endpoint" depth="3">
              {{
                profile.rule.mode === "dynamic"
                  ? t("SOCKS5 → 由应用指定目标")
                  : t("→ {value1} {value2}", {
                      value1:
                        profile.rule.mode === "remote"
                          ? t("本机侧")
                          : t("远程侧"),
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
                {{ t("停止") }}
              </NButton>
              <NButton
                v-else
                type="primary"
                :disabled="!hostName(profile)"
                @click="forwards.start(profile.id)"
              >
                {{ t("启动") }}
              </NButton>
              <NFlex :size="4">
                <NButton
                  size="small"
                  quaternary
                  :disabled="forwards.active(profile.id)"
                  @click="edit(profile)"
                >
                  {{ t("编辑") }}
                </NButton>
                <NButton size="small" quaternary @click="edit(profile, true)">
                  {{ t("复制") }}
                </NButton>
                <NButton
                  size="small"
                  quaternary
                  :disabled="forwards.active(profile.id)"
                  @click="remove(profile)"
                >
                  {{ t("删除") }}
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
