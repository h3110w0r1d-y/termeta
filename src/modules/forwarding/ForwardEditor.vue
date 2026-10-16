<script setup lang="ts">
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
      name: profile ? profile.name + (props.copy ? " 副本" : "") : "",
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
    requiredText("名称"),
    { max: 128, message: "名称最多128个字符", trigger: ["input", "blur"] },
  ],
  profileId: {
    required: true,
    trigger: "change",
    validator: (_, value) =>
      hostOptions.value.some((h) => h.value === value) ||
      new Error("请选择已保存的 SSH 主机"),
  },
  "rule.bindAddress": requiredText("监听 IP 地址"),
  "rule.bindPort": integerRange("监听端口", 1, 65535),
  ...(draft.rule.mode === "dynamic"
    ? {}
    : {
        "rule.targetAddress": requiredText("目标地址"),
        "rule.targetPort": integerRange("目标端口", 1, 65535),
      }),
}));
const description = computed(
  () =>
    ({
      local:
        "监听本机端口，经 SSH 服务器访问固定目标。目标地址从 SSH 服务器的网络环境解析。",
      remote:
        "在 SSH 服务器上监听端口，将流量转到本机可访问的固定目标。目标地址从本机的网络环境解析。",
      dynamic:
        "在本机提供 SOCKS5 代理，由应用指定目标，域名经 SSH 服务器解析。仅支持 TCP CONNECT，无认证，只允许监听本机回环 IP。",
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
    :title="draft.id ? '编辑端口转发' : '新建端口转发'"
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
      <NFormItem label="名称" path="name">
        <NInput
          v-model:value="draft.name"
          :maxlength="128"
          placeholder="例如：数据库、开发服务或代理"
        />
      </NFormItem>
      <NFormItem label="SSH 主机" path="profileId">
        <NSelect
          v-model:value="draft.profileId"
          :options="hostOptions"
          filterable
          placeholder="选择已保存的 SSH 主机"
        />
      </NFormItem>
      <NAlert v-if="!hostOptions.length" type="info">
        请先在主机页创建 SSH 主机，再创建转发。
      </NAlert>
      <NFormItem label="转发模式" path="rule.mode">
        <NRadioGroup :value="draft.rule.mode" @update:value="changeMode">
          <NRadioButton value="local">本地转发</NRadioButton>
          <NRadioButton value="remote">远程转发</NRadioButton>
          <NRadioButton value="dynamic">动态 SOCKS5</NRadioButton>
        </NRadioGroup>
      </NFormItem>
      <p>
        <NText depth="3">{{ description }}</NText>
      </p>
      <NGrid :cols="2" :x-gap="16">
        <NFormItemGi
          :label="draft.rule.mode === 'remote' ? '远程监听 IP' : '本机监听 IP'"
          path="rule.bindAddress"
        >
          <NInput
            v-model:value="draft.rule.bindAddress"
            placeholder="127.0.0.1 或 ::1"
          />
        </NFormItemGi>
        <NFormItemGi label="监听端口" path="rule.bindPort">
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
              draft.rule.mode === 'remote' ? '本机侧目标地址' : '远程侧目标地址'
            "
            path="rule.targetAddress"
          >
            <NInput
              :value="draft.rule.targetAddress ?? ''"
              @update:value="draft.rule.targetAddress = $event"
              placeholder="域名或 IP，IPv6 不带方括号"
            />
          </NFormItemGi>
          <NFormItemGi label="目标端口" path="rule.targetPort">
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
        默认仅本机访问；监听 0.0.0.0 或 ::
        会开放给其他设备。远程监听范围还受服务器 GatewayPorts 设置控制。
      </NText>
    </NForm>
    <NAlert v-if="error" type="error">{{ error }}</NAlert>
    <template #footer>
      <NFlex justify="end">
        <NButton :disabled="busy" @click="emit('update:open', false)">
          取消
        </NButton>
        <NButton
          type="primary"
          :loading="busy"
          :disabled="!hostOptions.length"
          @click="save"
        >
          保存
        </NButton>
      </NFlex>
    </template>
  </AppDialog>
</template>
