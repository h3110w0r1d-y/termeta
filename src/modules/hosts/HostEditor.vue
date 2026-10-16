<script setup lang="ts">
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
  SshConfig,
} from "../../services/desktop/types";
import { useSettings } from "../settings/store";
import { builtinSchemes } from "../settings/themes";
import { useHosts } from "./store";
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
  duplicate?: boolean;
  initialGroupId?: string | null;
}>();
const emit = defineEmits<{
  "update:open": [boolean];
  connect: [SshConfig, string];
  saved: [];
  "after-leave": [];
}>();
const settings = useSettings();
const hosts = useHosts(),
  busy = ref(false),
  error = ref(""),
  auth = ref<"password" | "privateKey">("password"),
  tagText = ref(""),
  hostScheme = ref(""),
  savedRevision = ref<number | null>(null),
  committedHost = ref<HostProfile | null>(null),
  credentials = ref<CredentialMetadata[]>([]),
  credentialId = ref(""),
  secret = ref("");
const blank = (): HostDraft => ({
  id: null,
  icon: "ssh",
  name: "",
  address: "",
  port: 22,
  username: "",
  favorite: false,
  tags: [],
  groupId: props.initialGroupId ?? null,
  authPlan: [{ type: "password" }],
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
      if (props.duplicate) draft.value.name += " 副本";
      savedRevision.value = props.duplicate
        ? null
        : (props.host?.revision ?? null);
      const first = draft.value.authPlan.find(
        (step) => step.type === "password" || step.type === "privateKey",
      );
      auth.value = first?.type === "privateKey" ? "privateKey" : "password";
      credentialId.value = first?.credentialId ?? "";
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
const authOptions = [
  { value: "password" as const, label: "密码认证" },
  { value: "privateKey" as const, label: "公钥认证" },
];
const credentialOptions = computed(() => [
  {
    value: "",
    label:
      auth.value === "privateKey"
        ? "请选择私钥凭据"
        : "不引用凭据，使用主机专属密码",
  },
  ...credentials.value
    .filter((item) => item.status === "ready" && item.kind === auth.value)
    .map((item) => ({ value: item.id, label: item.label })),
]);
const schemeOptions = computed(() => [
  { value: "", label: "跟随全局默认" },
  ...[...builtinSchemes, ...settings.value.customSchemes].map((scheme) => ({
    value: scheme.id,
    label: scheme.name,
  })),
]);
const groupOptions = computed(() => [
  { value: "", label: "未分组" },
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
});
const rules = computed<FormRules>(() => ({
  "draft.name": props.quick ? [] : requiredText("名称"),
  "draft.address": requiredText("主机地址"),
  "draft.icon": requiredText("图标"),
  "draft.username": requiredText("用户名"),
  "draft.port": integerRange("端口", 1, 65535),
  auth: requiredText("认证方式"),
  credentialId: auth.value === "privateKey" ? requiredText("私钥凭据") : [],
}));
async function submit() {
  if (busy.value || (!committedHost.value && !(await validateForm(form.value))))
    return;
  busy.value = true;
  error.value = "";
  try {
    if (auth.value === "privateKey" && !credentialId.value)
      throw new Error("请先在凭据页创建并选择私钥凭据");
    // 主机只保存认证引用；专属密码单独传递，避免进入轻量配置状态。
    const first: AuthStep =
      auth.value === "privateKey"
        ? { type: "privateKey", credentialId: credentialId.value }
        : { type: "password", credentialId: credentialId.value || undefined };
    const plan: AuthStep[] = [first];
    draft.value = {
      ...draft.value,
      address: draft.value.address.trim(),
      username: draft.value.username.trim(),
      tags: tagText.value
        .split(/[,，]/)
        .map((t) => t.trim())
        .filter(Boolean),
      authPlan: plan,
    };
    if (props.quick)
      emit(
        "connect",
        {
          configVersion: 1,
          address: draft.value.address,
          port: draft.value.port,
          username: draft.value.username,
          authPlan: plan,
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
          auth.value === "password" && !credentialId.value ? secret.value : "",
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
      ? `主机已保存，配色保存失败；重试只保存配色，不会修改密码。${errorMessage(e)}`
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
    :title="quick ? '快速连接' : host && !duplicate ? '编辑主机' : '新建主机'"
    description="选择密码或公钥认证；额外验证由服务器在连接时自动提示。"
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
        <NFormItemGi :span="2" label="名称" path="draft.name">
          <AppInput
            id="host-name"
            v-model="formModel.draft.name"
            maxlength="120"
            placeholder="例如：生产环境 · 上海"
          />
        </NFormItemGi>
        <NFormItemGi v-if="!quick" :span="2" label="图标" path="draft.icon">
          <NSelect
            v-model:value="formModel.draft.icon"
            :options="systemIconOptions"
            :render-label="renderSystemLabel"
            :menu-props="systemMenuProps"
            aria-label="主机图标"
          />
          <template #feedback>
            {{
              host?.systemDetectionAttempted
                ? "系统识别已尝试，不会再次自动检测；可手动选择图标。"
                : "默认 SSH 图标将在首次连接时自动识别；手动选择其他图标后不自动覆盖。"
            }}
          </template>
        </NFormItemGi>
        <NFormItemGi label="主机地址" path="draft.address">
          <AppInput
            id="host-address"
            v-model="formModel.draft.address"
            maxlength="253"
            placeholder="example.com / IPv6"
          />
        </NFormItemGi>
        <NFormItemGi label="端口" path="draft.port">
          <AppNumberField
            id="host-port"
            v-model="formModel.draft.port"
            :min="1"
            :max="65535"
          />
        </NFormItemGi>
        <NFormItemGi label="用户名" path="draft.username">
          <AppInput
            id="host-username"
            v-model="formModel.draft.username"
            maxlength="256"
            autocomplete="off"
          />
        </NFormItemGi>
        <NFormItemGi label="认证方式" path="auth">
          <AppSelect
            id="host-auth"
            v-model="formModel.auth"
            :options="authOptions"
            :disabled="fieldsLocked"
          />
        </NFormItemGi>
        <NFormItemGi
          :span="2"
          :label="auth === 'privateKey' ? '私钥凭据' : '密码凭据'"
          path="credentialId"
        >
          <AppSelect
            id="host-credential"
            v-model="formModel.credentialId"
            :options="credentialOptions"
            :disabled="fieldsLocked"
          />
        </NFormItemGi>
        <NGi v-if="auth === 'password' && !credentialId" :span="2">
          <template v-if="!quick">
            <NFormItem label="主机专属密码（可选）">
              <AppInput
                id="host-password"
                v-model="formModel.secret"
                type="password"
                autocomplete="new-password"
                :disabled="fieldsLocked"
                placeholder="填写后随主机安全保存；留空则连接时询问"
              />
            </NFormItem>
          </template>
        </NGi>
        <NFormItemGi
          v-if="!quick"
          :span="2"
          label="主机终端配色"
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
          <NFormItemGi label="分组" path="groupId">
            <AppSelect
              id="host-group"
              v-model="formModel.groupId"
              :options="groupOptions"
              :disabled="fieldsLocked"
            />
          </NFormItemGi>
          <NFormItemGi label="标签" path="tagText">
            <AppInput
              id="host-tags"
              v-model="formModel.tagText"
              placeholder="生产, Linux"
            />
          </NFormItemGi>
          <NGi :span="2">
            <AppCheckbox
              v-model="formModel.draft.favorite"
              :disabled="fieldsLocked"
            >
              加入收藏
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
          {{ committedHost ? "关闭" : "取消" }}
        </AppButton>
        <AppButton
          type="submit"
          form="host-editor-form"
          class="primary"
          :disabled="busy"
        >
          {{
            busy
              ? "处理中…"
              : committedHost
                ? "重试保存配色"
                : quick
                  ? "连接"
                  : "保存主机"
          }}
        </AppButton>
      </NFlex>
    </template>
  </AppDialog>
</template>
<style scoped></style>
