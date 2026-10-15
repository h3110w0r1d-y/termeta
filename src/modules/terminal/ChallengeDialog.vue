<script setup lang="ts">
import { computed, ref, watch } from "vue";
import AppDialog from "../../components/ui/AppDialog.vue";
import { useWorkspace } from "../workspace/store";
import { desktop, errorMessage } from "../../services/desktop/client";
const workspace = useWorkspace(),
  current = computed(() => workspace.challenges[0]);
const answers = ref<string[]>([]),
  busy = ref(false),
  error = ref("");
watch(current, (c) => {
  answers.value = c?.type === "authChallenge" ? c.prompts.map(() => "") : [];
  error.value = "";
});
async function hostKey(decision: "reject" | "allowOnce" | "trustAndSave") {
  const c = current.value;
  if (c?.type !== "hostKeyChallenge") return;
  busy.value = true;
  try {
    await desktop.hostKey(c.sessionId, c.challengeId, decision);
    workspace.dismiss(c.sessionId);
  } catch (e) {
    error.value = errorMessage(e);
  } finally {
    busy.value = false;
  }
}
async function auth(cancel = false) {
  const c = current.value;
  if (c?.type !== "authChallenge") return;
  busy.value = true;
  const secrets = [...answers.value];
  answers.value = answers.value.map(() => "");
  try {
    if (cancel) await desktop.cancelAuth(c.sessionId, c.challengeId);
    else await desktop.auth(c, secrets, "never");
    workspace.dismiss(c.sessionId);
  } catch (e) {
    error.value = errorMessage(e);
  } finally {
    secrets.fill("");
    busy.value = false;
  }
}
</script>
<template>
  <AppDialog
    :open="!!current"
    :locked="true"
    :title="
      current?.type === 'hostKeyChallenge'
        ? '核验 SSH 主机身份'
        : current?.type === 'authChallenge'
          ? current.name || 'SSH 身份认证'
          : ''
    "
    ><template v-if="current?.type === 'hostKeyChallenge'"
      ><p>请通过可信渠道核对服务器指纹。只有核验后才会提交认证信息。</p>
      <dl class="fingerprint">
        <dt>主机</dt>
        <dd>{{ current.endpoint.host }}:{{ current.endpoint.port }}</dd>
        <dt>算法</dt>
        <dd>{{ current.algorithm }}</dd>
        <dt>SHA256 指纹</dt>
        <dd>{{ current.fingerprint }}</dd>
      </dl>
      <p class="muted">
        保存信任不会自动替换已经信任的不同密钥。指纹变更会被阻断。
      </p>
      <footer class="dialog-actions">
        <button :disabled="busy" @click="hostKey('reject')">拒绝</button
        ><button :disabled="busy" @click="hostKey('allowOnce')">
          仅本次信任</button
        ><button
          class="primary"
          :disabled="busy"
          @click="hostKey('trustAndSave')"
        >
          信任并保存
        </button>
      </footer></template
    >
    <form
      v-else-if="current?.type === 'authChallenge'"
      class="form-grid"
      @submit.prevent="auth()"
    >
      <p class="span-2">{{ current.instructions }}</p>
      <label
        v-for="(prompt, index) in current.prompts"
        :key="index"
        class="span-2"
        >{{ prompt.text
        }}<input
          v-model="answers[index]"
          :type="prompt.echo ? 'text' : 'password'"
          autocomplete="off"
          :autofocus="index === 0"
          maxlength="16384"
      /></label>
      <footer class="dialog-actions span-2">
        <button type="button" :disabled="busy" @click="auth(true)">
          取消连接</button
        ><button class="primary" :disabled="busy">继续</button>
      </footer>
    </form>
    <p v-if="error" class="error" role="alert">{{ error }}</p></AppDialog
  >
</template>
