<script setup lang="ts">
import { NFlex, NForm } from "naive-ui";
import { clone } from "../../services/data/clone";
import { computed, ref, watch } from "vue";
import AppDialog from "../../components/ui/AppDialog.vue";
import AppSelect from "../../components/ui/AppSelect.vue";
import AppCheckbox from "../../components/ui/AppCheckbox.vue";
import AppButton from "../../components/ui/AppButton.vue";
import AppInput from "../../components/ui/AppInput.vue";
import AppLabel from "../../components/ui/AppLabel.vue";
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
const props = defineProps<{
  open: boolean;
  host?: HostProfile;
  quick?: boolean;
  duplicate?: boolean;
}>();
const emit = defineEmits<{
  "update:open": [boolean];
  connect: [SshConfig, string];
  saved: [];
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
  name: "",
  address: "",
  port: 22,
  username: "",
  favorite: false,
  tags: [],
  groupId: null,
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
  ...hosts.groups.map((group) => ({ value: group.id, label: group.name })),
]);
const groupId = computed({
  get: () => draft.value.groupId ?? "",
  set: (value: string) => {
    draft.value.groupId = value || null;
  },
});
const valid = computed(
  () =>
    draft.value.address.trim() &&
    draft.value.username.trim() &&
    Number.isInteger(draft.value.port) &&
    draft.value.port > 0 &&
    draft.value.port < 65536 &&
    !(!props.quick && !draft.value.name.trim()) &&
    (auth.value !== "privateKey" || !!credentialId.value),
);
async function submit() {
  if (!valid.value || busy.value) return;
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
    :locked="busy"
    :title="quick ? '快速连接' : host && !duplicate ? '编辑主机' : '新建主机'"
    description="选择密码或公钥认证；额外验证由服务器在连接时自动提示。"
    @update:open="emit('update:open', $event)"
  >
    <NForm
      id="host-editor-form"
      :disabled="fieldsLocked"
      @submit.prevent="submit"
    >
      <fieldset class="host-fields form-grid" :disabled="fieldsLocked">
        <div class="field span-2">
          <AppLabel for="host-name">名称</AppLabel>
          <AppInput
            id="host-name"
            v-model="draft.name"
            :required="!quick"
            maxlength="120"
            placeholder="例如：生产环境 · 上海"
          />
        </div>
        <div class="field">
          <AppLabel for="host-address">主机地址</AppLabel>
          <AppInput
            id="host-address"
            v-model="draft.address"
            required
            maxlength="253"
            placeholder="example.com / IPv6"
          />
        </div>
        <div class="field">
          <AppLabel for="host-port">端口</AppLabel>
          <AppNumberField
            id="host-port"
            v-model="draft.port"
            :min="1"
            :max="65535"
            required
          />
        </div>
        <div class="field">
          <AppLabel for="host-username">用户名</AppLabel>
          <AppInput
            id="host-username"
            v-model="draft.username"
            required
            maxlength="256"
            autocomplete="off"
          />
        </div>
        <div class="field">
          <AppLabel for="host-auth">认证方式</AppLabel>
          <AppSelect
            id="host-auth"
            v-model="auth"
            :options="authOptions"
            :disabled="fieldsLocked"
          />
        </div>
        <p class="muted span-2 auth-hint">
          连接时自动识别服务器要求的后续认证，需要验证码时会提示输入。
        </p>
        <div class="field span-2">
          <AppLabel for="host-credential">{{
            auth === "privateKey" ? "私钥凭据" : "密码凭据"
          }}</AppLabel>
          <AppSelect
            id="host-credential"
            v-model="credentialId"
            :options="credentialOptions"
            :disabled="fieldsLocked"
          />
          <p v-if="auth === 'privateKey'" class="muted auth-hint">
            私钥及其可选解密密码统一在凭据页创建和管理。
          </p>
        </div>
        <div v-if="auth === 'password' && !credentialId" class="field span-2">
          <template v-if="!quick">
            <AppLabel for="host-password">主机专属密码（可选）</AppLabel>
            <AppInput
              id="host-password"
              v-model="secret"
              type="password"
              autocomplete="new-password"
              :disabled="fieldsLocked"
              placeholder="填写后随主机安全保存；留空则连接时询问"
            />
            <p class="muted auth-hint">
              此密码仅用于当前主机，不进入可复用凭据库。{{
                host?.hasPassword && !duplicate
                  ? "当前已保存密码；留空保存会移除旧密码，改为连接时询问。"
                  : duplicate && host?.hasPassword
                    ? "复制主机不会复制原主机的专属密码。"
                    : ""
              }}
            </p>
          </template>
          <p v-else class="muted auth-hint">
            快速连接不保存专属密码，本次连接时会询问。
          </p>
        </div>
        <div v-if="!quick" class="field span-2">
          <AppLabel for="host-scheme">主机终端配色</AppLabel>
          <AppSelect
            id="host-scheme"
            v-model="hostScheme"
            :options="schemeOptions"
            :disabled="fieldsLocked"
          />
        </div>
        <template v-if="!quick">
          <div class="field">
            <AppLabel for="host-group">分组</AppLabel>
            <AppSelect
              id="host-group"
              v-model="groupId"
              :options="groupOptions"
              :disabled="fieldsLocked"
            />
          </div>
          <div class="field">
            <AppLabel for="host-tags">标签</AppLabel>
            <AppInput
              id="host-tags"
              v-model="tagText"
              placeholder="生产, Linux"
            />
          </div>
          <AppCheckbox
            v-model="draft.favorite"
            class="span-2"
            :disabled="fieldsLocked"
            >加入收藏</AppCheckbox
          >
        </template>
        <p v-if="error" class="error span-2" role="alert">{{ error }}</p>
      </fieldset>
    </NForm>
    <template #footer>
      <NFlex justify="end">
        <AppButton :disabled="busy" @click="emit('update:open', false)">{{
          committedHost ? "关闭" : "取消"
        }}</AppButton>
        <AppButton
          type="submit"
          form="host-editor-form"
          class="primary"
          :disabled="busy || !valid"
          >{{
            busy
              ? "处理中…"
              : committedHost
                ? "重试保存配色"
                : quick
                  ? "连接"
                  : "保存主机"
          }}</AppButton
        >
      </NFlex>
    </template>
  </AppDialog>
</template>
<style scoped>
.host-fields {
  border: 0;
  padding: 0;
  margin: 0;
  min-width: 0;
}
.field {
  display: grid;
  gap: 6px;
  min-width: 0;
}
.auth-hint {
  margin: 0;
  font-size: 12px;
  line-height: 1.6;
}
</style>
