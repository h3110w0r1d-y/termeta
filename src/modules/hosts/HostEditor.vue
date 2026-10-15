<script setup lang="ts">
import { clone } from "../../services/data/clone";
import { computed, ref, watch } from "vue";
import AppDialog from "../../components/ui/AppDialog.vue";
import { desktop, errorMessage } from "../../services/desktop/client";
import type {
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
  auth = ref("password"),
  keyName = ref(""),
  tagText = ref(""),
  hostScheme = ref<string>(),
  savedRevision = ref<number | null>(null),
  interactive = ref(false),
  saveSecret = ref(false),
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
watch(
  () => props.open,
  (open) => {
    if (open) {
      draft.value = props.host
        ? {
            ...clone(props.host),
            id: props.duplicate ? null : props.host.id,
          }
        : blank();
      if (props.duplicate) draft.value.name += " 副本";
      savedRevision.value = props.duplicate
        ? null
        : (props.host?.revision ?? null);
      interactive.value =
        draft.value.authPlan.length > 1 &&
        draft.value.authPlan.some((p) => p.type === "keyboardInteractive");
      auth.value = draft.value.authPlan[0]?.type ?? "password";
      tagText.value = draft.value.tags.join(", ");
      keyName.value = auth.value === "privateKey" ? "已授权私钥" : "";
      error.value = "";
      hostScheme.value = props.host
        ? settings.value.hostSchemes[props.host.id]
        : undefined;
      secret.value = "";
      saveSecret.value = false;
    } else {
      secret.value = "";
    }
  },
);
const valid = computed(
  () =>
    draft.value.address.trim() &&
    draft.value.username.trim() &&
    Number.isInteger(draft.value.port) &&
    draft.value.port > 0 &&
    draft.value.port < 65536 &&
    !(!props.quick && !draft.value.name.trim()),
);
async function selectKey() {
  try {
    const file = await desktop.selectKey();
    if (file) {
      draft.value.authPlan = [
        { type: "privateKey", keyFileRef: file.keyFileRef },
      ];
      keyName.value = file.name;
    }
  } catch (e) {
    error.value = errorMessage(e);
  }
}
async function submit() {
  if (!valid.value) return;
  busy.value = true;
  error.value = "";
  try {
    const plan: import("../../services/desktop/types").AuthStep[] =
      auth.value === "privateKey"
        ? draft.value.authPlan.filter((p) => p.type === "privateKey")
        : [
            draft.value.authPlan.find((p) => p.type === auth.value) ?? {
              type: auth.value as "password" | "keyboardInteractive",
            },
          ];
    if (!plan.length) throw new Error("请先通过原生选择器授权私钥");
    if (interactive.value && auth.value !== "keyboardInteractive")
      plan.push({ type: "keyboardInteractive" });
    if (
      saveSecret.value &&
      secret.value &&
      auth.value !== "keyboardInteractive"
    ) {
      const credential = await desktop.saveCredential(
        auth.value === "privateKey" ? "passphrase" : "password",
        draft.value.name || draft.value.address,
        secret.value,
      );
      const first = plan[0];
      if (first?.type === "password") first.credentialId = credential.id;
      else if (first?.type === "privateKey")
        first.passphraseCredentialId = credential.id;
      secret.value = "";
    }
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
      const saved = await hosts.save(draft.value, savedRevision.value);
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
    error.value = errorMessage(e);
  } finally {
    secret.value = "";
    busy.value = false;
  }
}
</script>
<template>
  <AppDialog
    :open="open"
    :title="quick ? '快速连接' : host && !duplicate ? '编辑主机' : '新建主机'"
    description="密码在连接时输入；私钥由原生选择器授权。"
    @update:open="emit('update:open', $event)"
    ><form class="form-grid" @submit.prevent="submit">
      <label class="span-2"
        >名称<input
          v-model="draft.name"
          :required="!quick"
          maxlength="120"
          placeholder="例如：生产环境 · 上海"
      /></label>
      <label
        >主机地址<input
          v-model="draft.address"
          required
          maxlength="253"
          placeholder="example.com / IPv6" /></label
      ><label
        >端口<input
          v-model.number="draft.port"
          type="number"
          min="1"
          max="65535"
          required
      /></label>
      <label
        >用户名<input
          v-model="draft.username"
          required
          maxlength="256"
          autocomplete="off" /></label
      ><label
        >认证方式<select v-model="auth">
          <option value="password">密码</option>
          <option value="privateKey">OpenSSH 私钥</option>
          <option value="keyboardInteractive">交互认证 / OTP</option>
        </select></label
      >
      <label v-if="auth !== 'keyboardInteractive'" class="check span-2"
        ><input v-model="interactive" type="checkbox" />密码 /
        私钥之后继续交互认证（OTP）</label
      >
      <div v-if="auth === 'privateKey'" class="span-2">
        <button type="button" @click="selectKey">选择私钥文件</button>
        <span class="muted">{{ keyName || "不读取或展示私钥正文" }}</span>
      </div>
      <div v-if="!quick && auth !== 'keyboardInteractive'" class="span-2">
        <label class="check"
          ><input v-model="saveSecret" type="checkbox" />保存密码 /
          私钥口令到系统凭据库</label
        ><input
          v-if="saveSecret"
          v-model="secret"
          type="password"
          autocomplete="new-password"
          placeholder="系统凭据库不可用时将明确报错"
        />
      </div>
      <label v-if="!quick" class="span-2"
        >主机终端配色<select v-model="hostScheme">
          <option :value="undefined">跟随全局默认</option>
          <option
            v-for="scheme in [
              ...builtinSchemes,
              ...settings.value.customSchemes,
            ]"
            :key="scheme.id"
            :value="scheme.id"
          >
            {{ scheme.name }}
          </option>
        </select></label
      >
      <template v-if="!quick"
        ><label
          >分组<select v-model="draft.groupId">
            <option :value="null">未分组</option>
            <option
              v-for="group in hosts.groups"
              :key="group.id"
              :value="group.id"
            >
              {{ group.name }}
            </option>
          </select></label
        ><label>标签<input v-model="tagText" placeholder="生产, Linux" /></label
        ><label class="check span-2"
          ><input v-model="draft.favorite" type="checkbox" />加入收藏</label
        ></template
      >
      <p v-if="error" class="error span-2" role="alert">{{ error }}</p>
      <footer class="dialog-actions span-2">
        <button type="button" @click="emit('update:open', false)">取消</button
        ><button class="primary" :disabled="busy || !valid">
          {{ busy ? "处理中…" : quick ? "连接" : "保存主机" }}
        </button>
      </footer>
    </form></AppDialog
  >
</template>
