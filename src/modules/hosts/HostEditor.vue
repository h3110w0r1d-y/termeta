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
        draft.value.name = t("common.format.copyName", {
          name: draft.value.name,
        });
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
  { value: "password" as const, label: t("hosts.auth.password") },
  { value: "privateKey" as const, label: t("hosts.auth.publicKey") },
]);
const startupOptions = computed(() => [
  { value: "shell", label: t("hosts.startup.shell") },
  { value: "exec", label: t("hosts.startup.command") },
]);
const credentialOptions = computed(() => [
  {
    value: "",
    label:
      auth.value === "privateKey"
        ? t("hosts.validation.privateKeyRequired")
        : t("hosts.form.useHostPassword"),
  },
  ...credentials.value
    .filter((item) => item.status === "ready" && item.kind === auth.value)
    .map((item) => ({ value: item.id, label: item.label })),
]);
const schemeOptions = computed(() => [
  { value: "", label: t("common.options.globalDefault") },
  ...[...builtinSchemes, ...settings.value.customSchemes].map((scheme) => ({
    value: scheme.id,
    label: scheme.name,
  })),
]);
const groupOptions = computed(() => [
  { value: "", label: t("hosts.groups.ungrouped") },
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
        requiredText("common.fields.name"),
        uniqueHostNameRule(
          () => hosts.hosts,
          () => draft.value.id,
        ),
      ],
  "draft.address": requiredText("hosts.form.address"),
  "draft.icon": requiredText("hosts.form.icon"),
  "draft.username": isTelnet.value
    ? []
    : requiredText("common.fields.username"),
  "draft.port": integerRange("common.fields.port", 1, 65535),
  startupType: isTelnet.value ? [] : requiredText("hosts.form.startupMode"),
  startupCommand:
    !isTelnet.value && startupType.value === "exec"
      ? [
          requiredText("hosts.form.startupCommand"),
          {
            trigger: ["input", "change", "blur"],
            validator: (_, value: string) =>
              (!value.includes("\0") &&
                new TextEncoder().encode(value).length <= 8192) ||
              new Error(t("hosts.validation.invalidCommand")),
          },
        ]
      : [],
  auth: isTelnet.value ? [] : requiredText("hosts.form.authentication"),
  credentialId:
    !isTelnet.value && auth.value === "privateKey"
      ? requiredText("hosts.form.privateKeyCredential")
      : [],
}));
async function submit() {
  if (busy.value || (!committedHost.value && !(await validateForm(form.value))))
    return;
  busy.value = true;
  error.value = "";
  try {
    if (!isTelnet.value && auth.value === "privateKey" && !credentialId.value)
      throw new Error(t("hosts.form.privateKeyHint"));
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
      ? t("hosts.errors.themeSave", {
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
      t('common.format.pair', {
        value1: quick
          ? t('hosts.actions.quickConnect')
          : host && !duplicate
            ? t('hosts.actions.edit')
            : t('hosts.actions.new'),
        value2: isTelnet ? 'Telnet' : 'SSH',
      })
    "
    :description="
      isTelnet ? t('hosts.telnet.warning') : t('hosts.form.description')
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
        <NFormItemGi
          :span="2"
          :label="t('common.fields.name')"
          path="draft.name"
        >
          <AppInput
            id="host-name"
            v-model="formModel.draft.name"
            maxlength="120"
            :placeholder="t('hosts.form.namePlaceholder')"
          />
        </NFormItemGi>
        <NFormItemGi
          v-if="!quick && !isTelnet"
          :span="2"
          :label="t('hosts.form.icon')"
          path="draft.icon"
        >
          <NSelect
            v-model:value="formModel.draft.icon"
            :options="systemIconOptions"
            :render-label="renderSystemLabel"
            :menu-props="systemMenuProps"
            :aria-label="t('hosts.form.iconLabel')"
          />
        </NFormItemGi>
        <NFormItemGi :label="t('hosts.form.address')" path="draft.address">
          <AppInput
            id="host-address"
            v-model="formModel.draft.address"
            maxlength="253"
            placeholder="example.com / IPv6"
          />
        </NFormItemGi>
        <NFormItemGi :label="t('common.fields.port')" path="draft.port">
          <AppNumberField
            id="host-port"
            v-model="formModel.draft.port"
            :min="1"
            :max="65535"
          />
        </NFormItemGi>
        <NFormItemGi
          v-if="!isTelnet"
          :label="t('common.fields.username')"
          path="draft.username"
        >
          <AppInput
            id="host-username"
            v-model="formModel.draft.username"
            maxlength="256"
            autocomplete="off"
          />
        </NFormItemGi>
        <NFormItemGi
          v-if="!isTelnet"
          :label="t('hosts.form.authentication')"
          path="auth"
        >
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
          :label="
            auth === 'privateKey'
              ? t('hosts.form.privateKeyCredential')
              : t('hosts.form.passwordCredential')
          "
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
            <NFormItem :label="t('hosts.form.password')">
              <AppInput
                id="host-password"
                v-model="formModel.secret"
                type="password"
                autocomplete="new-password"
                :disabled="fieldsLocked"
                :placeholder="t('hosts.form.passwordHint')"
              />
            </NFormItem>
          </template>
        </NGi>
        <NFormItemGi
          v-if="!isTelnet"
          :span="2"
          :label="t('hosts.form.startupMode')"
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
          :label="t('hosts.form.startupCommand')"
          path="startupCommand"
        >
          <AppInput
            id="host-startup-command"
            v-model="formModel.startupCommand"
            :disabled="fieldsLocked"
            :maxlength="8192"
            :placeholder="t('hosts.form.commandPlaceholder')"
            autocomplete="off"
          />
        </NFormItemGi>
        <NFormItemGi
          v-if="!quick"
          :span="2"
          :label="t('hosts.form.theme')"
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
          <NFormItemGi :label="t('common.fields.group')" path="groupId">
            <AppSelect
              id="host-group"
              v-model="formModel.groupId"
              :options="groupOptions"
              :disabled="fieldsLocked"
            />
          </NFormItemGi>
          <NFormItemGi :label="t('common.fields.tags')" path="tagText">
            <AppInput
              id="host-tags"
              v-model="formModel.tagText"
              :placeholder="t('hosts.form.tagsPlaceholder')"
            />
          </NFormItemGi>
          <NGi :span="2">
            <AppCheckbox
              v-model="formModel.draft.favorite"
              :disabled="fieldsLocked"
            >
              {{ t("common.actions.addFavorite") }}
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
          {{
            committedHost
              ? t("common.actions.close")
              : t("common.actions.cancel")
          }}
        </AppButton>
        <AppButton
          type="submit"
          form="host-editor-form"
          class="primary"
          :disabled="busy"
        >
          {{
            busy
              ? t("common.status.processing")
              : committedHost
                ? t("hosts.actions.retryTheme")
                : quick
                  ? t("common.actions.connect")
                  : t("hosts.actions.save")
          }}
        </AppButton>
      </NFlex>
    </template>
  </AppDialog>
</template>
<style scoped></style>
