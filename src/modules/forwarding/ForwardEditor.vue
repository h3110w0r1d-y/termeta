<script setup lang="ts">
import { t } from "../../i18n";
import { computed, reactive, ref, watch } from "vue";
import {
  NAlert,
  NButton,
  NFlex,
  NForm,
  NFormItem,
  NGrid,
  NFormItemGi,
  NInput,
  NInputNumber,
  NRadioGroup,
  NRadioButton,
  NSelect,
  NText,
  type FormInst,
  type FormRules,
} from "naive-ui";
import AppDialog from "../../components/ui/AppDialog.vue";
import {
  integerRange,
  requiredText,
  validateForm,
} from "../../services/forms/rules";
import { errorMessage } from "../../services/desktop/client";
import type {
  ForwardDraft,
  ForwardMode,
  ForwardProfile,
} from "../../services/desktop/types";
import { useHosts } from "../hosts/store";
import { usePortForwards } from "./store";
const props = defineProps<{
  open: boolean;
  profile?: ForwardProfile;
  copy?: boolean;
}>();
const emit = defineEmits<{ "update:open": [boolean] }>();
const hosts = useHosts(),
  forwards = usePortForwards();
const draft = reactive<ForwardDraft>({
  id: null,
  name: "",
  profileId: "",
  rule: {
    mode: "local",
    bindAddress: "127.0.0.1",
    bindPort: 8080,
    targetAddress: "127.0.0.1",
    targetPort: 80,
  },
});
const form = ref<FormInst>(),
  busy = ref(false),
  error = ref("");
const hostOptions = computed(() =>
  hosts.hosts
    .filter((h) => h.kind === "ssh")
    .map((h) => ({
      label: `${h.name} (${h.username}@${h.address}:${h.port})`,
      value: h.id,
    })),
);
watch(
  () => props.open,
  (open) => {
    if (!open) return;
    const profile = props.profile;
    Object.assign(draft, {
      id: props.copy ? null : (profile?.id ?? null),
      name: profile
        ? props.copy
          ? t("{name} 副本", { name: profile.name })
          : profile.name
        : "",
      profileId: profile?.profileId ?? "",
      rule: profile
        ? { ...profile.rule }
        : {
            mode: "local",
            bindAddress: "127.0.0.1",
            bindPort: 8080,
            targetAddress: "127.0.0.1",
            targetPort: 80,
          },
    });
    error.value = "";
    form.value?.restoreValidation();
  },
);
function changeMode(mode: string | number | boolean) {
  draft.rule.mode = mode as ForwardMode;
  if (mode === "dynamic" && !props.profile) draft.rule.bindPort = 1080;
  if (mode !== "dynamic") {
    draft.rule.targetAddress ??= "127.0.0.1";
    draft.rule.targetPort ??= 80;
  }
  form.value?.restoreValidation();
}
const rules = computed<FormRules>(() => ({
  name: [
    requiredText(t("名称")),
    { max: 128, message: t("名称最多128个字符"), trigger: ["input", "blur"] },
  ],
  profileId: {
    required: true,
    trigger: "change",
    validator: (_, value) =>
      hostOptions.value.some((h) => h.value === value) ||
      new Error(t("请选择已保存的 SSH 主机")),
  },
  "rule.bindAddress": requiredText(t("监听 IP 地址")),
  "rule.bindPort": integerRange(t("监听端口"), 1, 65535),
  ...(draft.rule.mode === "dynamic"
    ? {}
    : {
        "rule.targetAddress": requiredText(t("目标地址")),
        "rule.targetPort": integerRange(t("目标端口"), 1, 65535),
      }),
}));
const description = computed(
  () =>
    ({
      local: t("通过本机端口访问远程目标，目标地址由 SSH 服务器解析。"),
      remote: t("通过服务器端口访问本机侧目标，目标地址由本机解析。"),
      dynamic: t(
        "创建仅本机可用的 SOCKS5 代理，由 SSH 服务器解析域名。仅支持 TCP，无认证。",
      ),
    })[draft.rule.mode],
);
async function save() {
  if (busy.value || !(await validateForm(form.value))) return;
  busy.value = true;
  error.value = "";
  try {
    const rule = {
      ...draft.rule,
      bindAddress: draft.rule.bindAddress.trim(),
      targetAddress:
        draft.rule.mode === "dynamic"
          ? null
          : (draft.rule.targetAddress?.trim() ?? ""),
      targetPort: draft.rule.mode === "dynamic" ? null : draft.rule.targetPort,
    };
    await forwards.save(
      { ...draft, name: draft.name.trim(), rule },
      draft.id ? props.profile!.revision : null,
    );
    emit("update:open", false);
  } catch (cause) {
    error.value = errorMessage(cause);
  } finally {
    busy.value = false;
  }
}
</script>
<template>
  <AppDialog
    :open="open"
    :title="draft.id ? t('编辑端口转发') : t('新建端口转发')"
    :locked="busy"
    @update:open="emit('update:open', $event)"
  >
    <NForm
      ref="form"
      :model="draft"
      :rules="rules"
      :disabled="busy"
      @submit.prevent="save"
    >
      <NFormItem :label="t('名称')" path="name">
        <NInput
          v-model:value="draft.name"
          :maxlength="128"
          :placeholder="t('例如：数据库、开发服务或代理')"
        />
      </NFormItem>
      <NFormItem :label="t('SSH 主机')" path="profileId">
        <NSelect
          v-model:value="draft.profileId"
          :options="hostOptions"
          filterable
          :placeholder="t('选择已保存的 SSH 主机')"
        />
      </NFormItem>
      <NAlert v-if="!hostOptions.length" type="info">
        {{ t("请先在主机页创建 SSH 主机，再创建转发。") }}
      </NAlert>
      <NFormItem :label="t('转发模式')" path="rule.mode">
        <NRadioGroup :value="draft.rule.mode" @update:value="changeMode">
          <NRadioButton value="local">{{ t("本地转发") }}</NRadioButton>
          <NRadioButton value="remote">{{ t("远程转发") }}</NRadioButton>
          <NRadioButton value="dynamic">{{ t("动态 SOCKS5") }}</NRadioButton>
        </NRadioGroup>
      </NFormItem>
      <p>
        <NText depth="3">{{ description }}</NText>
      </p>
      <NGrid :cols="2" :x-gap="16">
        <NFormItemGi
          :label="
            draft.rule.mode === 'remote' ? t('远程监听 IP') : t('本机监听 IP')
          "
          path="rule.bindAddress"
        >
          <NInput
            v-model:value="draft.rule.bindAddress"
            :placeholder="t('127.0.0.1 或 ::1')"
          />
        </NFormItemGi>
        <NFormItemGi :label="t('监听端口')" path="rule.bindPort">
          <NInputNumber
            v-model:value="draft.rule.bindPort"
            :min="1"
            :max="65535"
            :precision="0"
          />
        </NFormItemGi>
        <template v-if="draft.rule.mode !== 'dynamic'">
          <NFormItemGi
            :label="
              draft.rule.mode === 'remote'
                ? t('本机侧目标地址')
                : t('远程侧目标地址')
            "
            path="rule.targetAddress"
          >
            <NInput
              :value="draft.rule.targetAddress ?? ''"
              @update:value="draft.rule.targetAddress = $event"
              :placeholder="t('域名或 IP，IPv6 不带方括号')"
            />
          </NFormItemGi>
          <NFormItemGi :label="t('目标端口')" path="rule.targetPort">
            <NInputNumber
              v-model:value="draft.rule.targetPort"
              :min="1"
              :max="65535"
              :precision="0"
            />
          </NFormItemGi>
        </template>
      </NGrid>
      <NText v-if="draft.rule.mode !== 'dynamic'" depth="3">
        {{
          t(
            "监听 0.0.0.0 或 :: 将允许其他设备访问。远程监听范围还取决于服务器设置。",
          )
        }}
      </NText>
    </NForm>
    <NAlert v-if="error" type="error">{{ error }}</NAlert>
    <template #footer>
      <NFlex justify="end">
        <NButton :disabled="busy" @click="emit('update:open', false)">
          {{ t("取消") }}
        </NButton>
        <NButton
          type="primary"
          :loading="busy"
          :disabled="!hostOptions.length"
          @click="save"
        >
          {{ t("保存") }}
        </NButton>
      </NFlex>
    </template>
  </AppDialog>
</template>
