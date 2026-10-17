<script setup lang="ts">
import { t } from "../../i18n";
import {
  NFlex,
  NSelect,
  type SelectRenderLabel,
  NForm,
  NFormItem,
  NFormItemGi,
  NGrid,
  NGi,
  type FormInst,
  type FormRules,
} from "naive-ui";
import { clone } from "../../services/data/clone";
import { computed, ref, watch, reactive, h } from "vue";
import AppDialog from "../../components/ui/AppDialog.vue";
import AppSelect from "../../components/ui/AppSelect.vue";
import AppCheckbox from "../../components/ui/AppCheckbox.vue";
import AppButton from "../../components/ui/AppButton.vue";
import AppInput from "../../components/ui/AppInput.vue";
import {
  requiredText,
  integerRange,
  validateForm,
} from "../../services/forms/rules";
import AppNumberField from "../../components/ui/AppNumberField.vue";
import { desktop, errorMessage } from "../../services/desktop/client";
import type {
  AuthStep,
  CredentialMetadata,
  HostDraft,
  HostProfile,
  OpenTarget,
  SshStartup,
} from "../../services/desktop/types";
import { useSettings } from "../settings/store";
import { builtinSchemes } from "../settings/themes";
import { useHosts } from "./store";
import { uniqueHostNameRule } from "./validation";
import { groupChoices } from "./groups";
import HostSystemIcon from "./HostSystemIcon.vue";
import { systemIconOptions } from "./systemIcons";
const systemMenuProps = { class: "system-icon-menu", "data-ui-popup-open": "" };
const renderSystemLabel: SelectRenderLabel = (option) =>
  h(NFlex, { align: "center", size: 8, inline: true, wrap: false }, () => [
    h(HostSystemIcon, { icon: String(option.value), size: 20 }),
    h("span", String(option.label)),
  ]);
const props = defineProps<{
  open: boolean;
  host?: HostProfile;
  quick?: boolean;
  kind?: "ssh" | "telnet";
  duplicate?: boolean;
  initialGroupId?: string | null;
}>();
const emit = defineEmits<{
  "update:open": [boolean];
  connect: [OpenTarget, string];
  saved: [];
  "after-leave": [];
}>();
const isTelnet = computed(() => (props.host?.kind ?? props.kind) === "telnet");
const settings = useSettings();
const hosts = useHosts(),
  busy = ref(false),
  error = ref(""),
  auth = ref<"password" | "privateKey">("privateKey"),
  tagText = ref(""),
  hostScheme = ref(""),
  savedRevision = ref<number | null>(null),
  committedHost = ref<HostProfile | null>(null),
  credentials = ref<CredentialMetadata[]>([]),
  credentialId = ref(""),
  secret = ref("");
const startupType = ref<SshStartup["type"]>("shell");
const startupCommand = ref("");
const blank = (): HostDraft => ({
  id: null,
  kind: isTelnet.value ? "telnet" : "ssh",
  serialConfig: undefined,
  icon: "ssh",
  name: "",
  address: "",
  port: isTelnet.value ? 23 : 22,
  username: "",
  favorite: false,
  tags: [],
  groupId: props.initialGroupId ?? null,
  authPlan: isTelnet.value ? [] : [{ type: "privateKey", credentialId: "" }],
});
const draft = ref<HostDraft>(blank());
const fieldsLocked = computed(() => busy.value || !!committedHost.value);
watch(
  () => props.open,
  (open) => {
    if (open) {
      committedHost.value = null;
      draft.value = props.host
        ? (() => {
            const {
              hasPassword: _hasPassword,
              systemDetectionAttempted: _systemDetectionAttempted,
              revision: _revision,
              ...profile
            } = clone(props.host);
            return { ...profile, id: props.duplicate ? null : props.host.id };
          })()
        : blank();
      if (props.duplicate)
        draft.value.name = t("{name} 副本", { name: draft.value.name });
      const startup = draft.value.startup;
      startupType.value = startup?.type ?? "shell";
      startupCommand.value = startup?.type === "exec" ? startup.command : "";
      savedRevision.value = props.duplicate
        ? null
        : (props.host?.revision ?? null);
      const first = draft.value.authPlan.find(
        (step) => step.type === "password" || step.type === "privateKey",
      );
      auth.value = first?.type === "privateKey" ? "privateKey" : "password";
      credentialId.value = first?.credentialId ?? "";
      if (!isTelnet.value)
        void desktop
          .listCredentials()
          .then((items) => {
            credentials.value = items;
          })
          .catch((e) => {
            error.value = errorMessage(e);
          });
      tagText.value = draft.value.tags.join(", ");
      error.value = "";
      hostScheme.value = props.host
        ? (settings.value.hostSchemes[props.host.id] ?? "")
        : "";
      secret.value = "";
    } else {
      secret.value = "";
    }
  },
);
watch(
  auth,
  () => {
    credentialId.value = "";
    secret.value = "";
  },
  { flush: "sync" },
);
watch(
  credentialId,
  () => {
    secret.value = "";
  },
  { flush: "sync" },
);
const authOptions = computed(() => [
  { value: "password" as const, label: t("密码认证") },
  { value: "privateKey" as const, label: t("公钥认证") },
]);
const startupOptions = computed(() => [
  { value: "shell", label: t("交互 Shell") },
  { value: "exec", label: t("执行命令") },
]);
const credentialOptions = computed(() => [
  {
    value: "",
    label:
      auth.value === "privateKey" ? t("请选择私钥凭据") : t("使用此主机的密码"),
  },
  ...credentials.value
    .filter((item) => item.status === "ready" && item.kind === auth.value)
    .map((item) => ({ value: item.id, label: item.label })),
]);
const schemeOptions = computed(() => [
  { value: "", label: t("跟随全局默认") },
  ...[...builtinSchemes, ...settings.value.customSchemes].map((scheme) => ({
    value: scheme.id,
    label: scheme.name,
  })),
]);
const groupOptions = computed(() => [
  { value: "", label: t("未分组") },
  ...groupChoices(hosts.groups),
]);
const groupId = computed({
  get: () => draft.value.groupId ?? "",
  set: (value: string) => {
    draft.value.groupId = value || null;
  },
});
const form = ref<FormInst>();
// 表单与控件共享响应式字段；保留 ref 引用，重置草稿时无需等待重新渲染。
const formModel = reactive({
  draft,
  auth,
  credentialId,
  secret,
  hostScheme,
  groupId,
  tagText,
  startupType,
  startupCommand,
});
const rules = computed<FormRules>(() => ({
  "draft.name": props.quick
    ? []
    : [
        requiredText(t("名称")),
        uniqueHostNameRule(
          () => hosts.hosts,
          () => draft.value.id,
        ),
      ],
  "draft.address": requiredText(t("主机地址")),
  "draft.icon": requiredText(t("图标")),
  "draft.username": isTelnet.value ? [] : requiredText(t("用户名")),
  "draft.port": integerRange(t("端口"), 1, 65535),
  startupType: isTelnet.value ? [] : requiredText(t("启动方式")),
  startupCommand:
    !isTelnet.value && startupType.value === "exec"
      ? [
          requiredText(t("启动命令")),
          {
            trigger: ["input", "change", "blur"],
            validator: (_, value: string) =>
              (!value.includes("\0") &&
                new TextEncoder().encode(value).length <= 8192) ||
              new Error(t("命令过长或包含无效字符")),
          },
        ]
      : [],
  auth: isTelnet.value ? [] : requiredText(t("认证方式")),
  credentialId:
    !isTelnet.value && auth.value === "privateKey"
      ? requiredText(t("私钥凭据"))
      : [],
}));
async function submit() {
  if (busy.value || (!committedHost.value && !(await validateForm(form.value))))
    return;
  busy.value = true;
  error.value = "";
  try {
    if (!isTelnet.value && auth.value === "privateKey" && !credentialId.value)
      throw new Error(t("请先在凭据页创建并选择私钥凭据"));
    // 主机只保存认证引用；专属密码单独传递，避免进入轻量配置状态。
    const first: AuthStep =
      auth.value === "privateKey"
        ? { type: "privateKey", credentialId: credentialId.value }
        : { type: "password", credentialId: credentialId.value || undefined };
    const plan: AuthStep[] = isTelnet.value ? [] : [first];
    draft.value = {
      ...draft.value,
      address: draft.value.address.trim(),
      username: isTelnet.value ? "" : draft.value.username.trim(),
      tags: tagText.value
        .split(/[,，]/)
        .map((t) => t.trim())
        .filter(Boolean),
      authPlan: plan,
      startup:
        !isTelnet.value && startupType.value === "exec"
          ? { type: "exec", command: startupCommand.value }
          : { type: "shell" },
    };
    if (props.quick)
      emit(
        "connect",
        isTelnet.value
          ? {
              source: "telnet",
              config: {
                configVersion: 1,
                address: draft.value.address,
                port: draft.value.port,
              },
            }
          : {
              source: "ephemeral",
              kind: "ssh",
              config: {
                configVersion: 1,
                address: draft.value.address,
                port: draft.value.port,
                username: draft.value.username,
                authPlan: plan,
                startup: draft.value.startup,
              },
            },
        draft.value.name || draft.value.address,
      );
    else {
      // 主机提交后重试仅保存配色，不能因输入已清空再次覆盖专属密码。
      const saved =
        committedHost.value ??
        (await hosts.save(
          draft.value,
          savedRevision.value,
          isTelnet.value
            ? undefined
            : auth.value === "password" && !credentialId.value
              ? secret.value
              : "",
        ));
      committedHost.value = saved;
      savedRevision.value = saved.revision;
      draft.value.id = saved.id;
      const next = clone(settings.value);
      if (hostScheme.value) next.hostSchemes[saved.id] = hostScheme.value;
      else delete next.hostSchemes[saved.id];
      await settings.save(next);
      emit("saved");
    }
    emit("update:open", false);
  } catch (e) {
    error.value = committedHost.value
      ? t("主机已保存，配色未保存。请重试：{value1}", {
          value1: errorMessage(e),
        })
      : errorMessage(e);
  } finally {
    secret.value = "";
    busy.value = false;
  }
}
</script>
<template>
  <AppDialog
    :open="open"
    @after-leave="emit('after-leave')"
    :locked="busy"
    :title="
      t('{value1} · {value2}', {
        value1: quick
          ? t('快速连接')
          : host && !duplicate
            ? t('编辑主机')
            : t('新建主机'),
        value2: isTelnet ? 'Telnet' : 'SSH',
      })
    "
    :description="
      isTelnet
        ? t(
            'Telnet 以明文传输，用户名和密码在终端内输入；请仅连接可信网络中的设备。',
          )
        : t('使用密码或公钥连接主机。')
    "
    @update:open="emit('update:open', $event)"
  >
    <NForm
      ref="form"
      :model="formModel"
      :rules="rules"
      label-placement="top"
      novalidate
      id="host-editor-form"
      :disabled="fieldsLocked"
      @submit.prevent="submit"
    >
      <NGrid :cols="2" :x-gap="16">
        <NFormItemGi :span="2" :label="t('名称')" path="draft.name">
          <AppInput
            id="host-name"
            v-model="formModel.draft.name"
            maxlength="120"
            :placeholder="t('例如：生产环境 · 上海')"
          />
        </NFormItemGi>
        <NFormItemGi
          v-if="!quick && !isTelnet"
          :span="2"
          :label="t('图标')"
          path="draft.icon"
        >
          <NSelect
            v-model:value="formModel.draft.icon"
            :options="systemIconOptions"
            :render-label="renderSystemLabel"
            :menu-props="systemMenuProps"
            :aria-label="t('主机图标')"
          />
        </NFormItemGi>
        <NFormItemGi :label="t('主机地址')" path="draft.address">
          <AppInput
            id="host-address"
            v-model="formModel.draft.address"
            maxlength="253"
            placeholder="example.com / IPv6"
          />
        </NFormItemGi>
        <NFormItemGi :label="t('端口')" path="draft.port">
          <AppNumberField
            id="host-port"
            v-model="formModel.draft.port"
            :min="1"
            :max="65535"
          />
        </NFormItemGi>
        <NFormItemGi
          v-if="!isTelnet"
          :label="t('用户名')"
          path="draft.username"
        >
          <AppInput
            id="host-username"
            v-model="formModel.draft.username"
            maxlength="256"
            autocomplete="off"
          />
        </NFormItemGi>
        <NFormItemGi v-if="!isTelnet" :label="t('认证方式')" path="auth">
          <AppSelect
            id="host-auth"
            v-model="formModel.auth"
            :options="authOptions"
            :disabled="fieldsLocked"
          />
        </NFormItemGi>
        <NFormItemGi
          v-if="!isTelnet"
          :span="2"
          :label="auth === 'privateKey' ? t('私钥凭据') : t('密码凭据')"
          path="credentialId"
        >
          <AppSelect
            id="host-credential"
            v-model="formModel.credentialId"
            :options="credentialOptions"
            :disabled="fieldsLocked"
          />
        </NFormItemGi>
        <NGi v-if="!isTelnet && auth === 'password' && !credentialId" :span="2">
          <template v-if="!quick">
            <NFormItem :label="t('主机专属密码（可选）')">
              <AppInput
                id="host-password"
                v-model="formModel.secret"
                type="password"
                autocomplete="new-password"
                :disabled="fieldsLocked"
                :placeholder="t('填写后随主机安全保存；留空则连接时询问')"
              />
            </NFormItem>
          </template>
        </NGi>
        <NFormItemGi
          v-if="!isTelnet"
          :span="2"
          :label="t('启动方式')"
          path="startupType"
        >
          <AppSelect
            id="host-startup"
            v-model="formModel.startupType"
            :options="startupOptions"
            :disabled="fieldsLocked"
          />
        </NFormItemGi>
        <NFormItemGi
          v-if="!isTelnet && startupType === 'exec'"
          :span="2"
          :label="t('启动命令')"
          path="startupCommand"
        >
          <AppInput
            id="host-startup-command"
            v-model="formModel.startupCommand"
            :disabled="fieldsLocked"
            :maxlength="8192"
            :placeholder="t('例如：/usr/bin/btop 或 docker stats')"
            autocomplete="off"
          />
        </NFormItemGi>
        <NFormItemGi
          v-if="!quick"
          :span="2"
          :label="t('主机终端配色')"
          path="hostScheme"
        >
          <AppSelect
            id="host-scheme"
            v-model="formModel.hostScheme"
            :options="schemeOptions"
            :disabled="fieldsLocked"
          />
        </NFormItemGi>
        <template v-if="!quick">
          <NFormItemGi :label="t('分组')" path="groupId">
            <AppSelect
              id="host-group"
              v-model="formModel.groupId"
              :options="groupOptions"
              :disabled="fieldsLocked"
            />
          </NFormItemGi>
          <NFormItemGi :label="t('标签')" path="tagText">
            <AppInput
              id="host-tags"
              v-model="formModel.tagText"
              :placeholder="t('生产, Linux')"
            />
          </NFormItemGi>
          <NGi :span="2">
            <AppCheckbox
              v-model="formModel.draft.favorite"
              :disabled="fieldsLocked"
            >
              {{ t("加入收藏") }}
            </AppCheckbox>
          </NGi>
        </template>
        <NGi v-if="error" :span="2">
          <p class="error" role="alert">{{ error }}</p>
        </NGi>
      </NGrid>
    </NForm>
    <template #footer>
      <NFlex justify="end">
        <AppButton :disabled="busy" @click="emit('update:open', false)">
          {{ committedHost ? t("关闭") : t("取消") }}
        </AppButton>
        <AppButton
          type="submit"
          form="host-editor-form"
          class="primary"
          :disabled="busy"
        >
          {{
            busy
              ? t("处理中…")
              : committedHost
                ? t("重试保存配色")
                : quick
                  ? t("连接")
                  : t("保存主机")
          }}
        </AppButton>
      </NFlex>
    </template>
  </AppDialog>
</template>
<style scoped></style>
