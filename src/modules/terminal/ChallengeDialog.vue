<script setup lang="ts">
import { NFlex } from "naive-ui";
import AppButton from "../../components/ui/AppButton.vue";
import AppInput from "../../components/ui/AppInput.vue";
import AppLabel from "../../components/ui/AppLabel.vue";
import { computed, nextTick, ref, watch } from "vue";
import AppDialog from "../../components/ui/AppDialog.vue";
import { useWorkspace } from "../workspace/store";
import { desktop, errorMessage } from "../../services/desktop/client";
const workspace = useWorkspace(),
  current = computed(() => workspace.challenges[0]),
  targetTitle = computed(
    () =>
      workspace.tabs.find((tab) => tab.sessionId === current.value?.sessionId)
        ?.title ?? workspace.fileSessionTitles[current.value?.sessionId ?? ""],
  );
const answers = ref<string[]>([]),
  authForm = ref<HTMLFormElement>(),
  busy = ref(false),
  error = ref("");
watch(current, async (c) => {
  answers.value = c?.type === "authChallenge" ? c.prompts.map(() => "") : [];
  error.value = "";
  await nextTick();
  if (
    current.value?.challengeId === c?.challengeId &&
    c?.type === "authChallenge"
  )
    (
      authForm.value?.querySelector<HTMLElement>("input") ??
      document.getElementById("auth-challenge-continue")
    )?.focus();
});
async function hostKey(decision: "reject" | "allowOnce" | "trustAndSave") {
  if (busy.value) return;
  const c = current.value;
  if (c?.type !== "hostKeyChallenge") return;
  busy.value = true;
  try {
    await desktop.hostKey(c.sessionId, c.challengeId, decision);
    // 只消费本次提示，避免迟到的提交响应删除服务器刚发来的下一轮认证。
    workspace.dismiss(c.sessionId, c.challengeId);
  } catch (e) {
    if (current.value?.challengeId === c.challengeId)
      error.value = errorMessage(e);
  } finally {
    busy.value = false;
  }
}
async function auth(cancel = false) {
  if (busy.value) return;
  const c = current.value;
  if (c?.type !== "authChallenge") return;
  busy.value = true;
  const secrets = [...answers.value];
  answers.value = answers.value.map(() => "");
  try {
    if (cancel) await desktop.cancelAuth(c.sessionId, c.challengeId);
    else await desktop.auth(c, secrets, "never");
    workspace.dismiss(c.sessionId, c.challengeId);
  } catch (e) {
    if (current.value?.challengeId === c.challengeId)
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
    :description="targetTitle ? `连接目标：${targetTitle}` : undefined"
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
    </template>
    <form
      v-else-if="current?.type === 'authChallenge'"
      ref="authForm"
      id="auth-challenge-form"
      class="form-grid"
      @submit.prevent="auth()"
    >
      <p class="span-2">{{ current.instructions }}</p>
      <AppLabel
        v-for="(prompt, index) in current.prompts"
        :key="index"
        class="span-2"
        >{{ prompt.text
        }}<AppInput
          v-model="answers[index]"
          :type="prompt.echo ? 'text' : 'password'"
          autocomplete="off"
          :autofocus="index === 0"
          maxlength="16384"
      /></AppLabel>
    </form>
    <p v-if="error" class="error" role="alert">{{ error }}</p>
    <template #footer>
      <NFlex v-if="current?.type === 'hostKeyChallenge'" justify="end">
        <AppButton :disabled="busy" @click="hostKey('reject')">拒绝</AppButton>
        <AppButton :disabled="busy" @click="hostKey('allowOnce')"
          >仅本次信任</AppButton
        >
        <AppButton
          class="primary"
          :disabled="busy"
          @click="hostKey('trustAndSave')"
          >信任并保存</AppButton
        >
      </NFlex>
      <NFlex v-else-if="current?.type === 'authChallenge'" justify="end">
        <AppButton type="button" :disabled="busy" @click="auth(true)">
          取消连接</AppButton
        ><AppButton
          id="auth-challenge-continue"
          type="submit"
          form="auth-challenge-form"
          class="primary"
          :disabled="busy"
          >继续</AppButton
        >
      </NFlex>
    </template></AppDialog
  >
</template>
