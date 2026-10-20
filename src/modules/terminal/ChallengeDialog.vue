<script setup lang="ts">
import { t } from "@/i18n";
import {
  NFlex,
  NForm,
  NFormItem,
  type FormInst,
  type FormRules,
} from "naive-ui";
import AppButton from "@/components/ui/AppButton.vue";
import AppInput from "@/components/ui/AppInput.vue";
import { validateForm } from "@/services/forms/rules";
import { computed, nextTick, ref, watch } from "vue";
import AppDialog from "@/components/ui/AppDialog.vue";
import { useWorkbenchStore } from "../workbench/store";
import { desktop, errorMessage } from "@/services/desktop/client";
const emit = defineEmits<{ "after-leave": [] }>();
const workbench = useWorkbenchStore(),
  current = computed(() => workbench.challenges[0]),
  targetTitle = computed(
    () =>
      workbench.terminals.find(
        (tab) => tab.sessionId === current.value?.sessionId,
      )?.title ?? workbench.connectionTitles[current.value?.sessionId ?? ""],
  );
const answers = ref<string[]>([]),
  authForm = ref<FormInst>(),
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
      document.querySelector<HTMLElement>("#auth-challenge-form input") ??
      document.getElementById("auth-challenge-continue")
    )?.focus();
});
const rules = computed<FormRules>(() =>
  Object.fromEntries(
    answers.value.map((_, index) => [
      String(index),
      {
        max: 16384,
        message: t("terminal.auth.answerTooLong"),
        trigger: ["input", "blur"],
      },
    ]),
  ),
);
async function hostKey(decision: "reject" | "allowOnce" | "trustAndSave") {
  if (busy.value) return;
  const c = current.value;
  if (c?.type !== "hostKeyChallenge") return;
  busy.value = true;
  try {
    await desktop.hostKey(c.sessionId, c.challengeId, decision);
    // 只消费本次提示，避免迟到的提交响应删除服务器刚发来的下一轮认证。
    workbench.dismiss(c.sessionId, c.challengeId);
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
  if (!cancel && !(await validateForm(authForm.value))) return;
  busy.value = true;
  const secrets = [...answers.value];
  answers.value = answers.value.map(() => "");
  try {
    if (cancel) await desktop.cancelAuth(c.sessionId, c.challengeId);
    else await desktop.auth(c, secrets, "never");
    workbench.dismiss(c.sessionId, c.challengeId);
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
    @after-leave="emit('after-leave')"
    :locked="true"
    :description="
      targetTitle
        ? t('terminal.auth.target', { targetTitle: targetTitle })
        : undefined
    "
    :title="
      current?.type === 'hostKeyChallenge'
        ? t('knownHosts.verifyTitle')
        : current?.type === 'authChallenge'
          ? current.name || t('terminal.auth.title')
          : ''
    "
  >
    <template v-if="current?.type === 'hostKeyChallenge'">
      <p>
        {{ t("knownHosts.verifyDescription") }}
      </p>
      <dl class="fingerprint">
        <dt>{{ t("hosts.form.address") }}</dt>
        <dd>{{ current.endpoint.host }}:{{ current.endpoint.port }}</dd>
        <dt>{{ t("knownHosts.algorithm") }}</dt>
        <dd>{{ current.algorithm }}</dd>
        <dt>{{ t("knownHosts.fingerprint") }}</dt>
        <dd>{{ current.fingerprint }}</dd>
      </dl>
      <p class="muted">
        {{ t("knownHosts.changedWarning") }}
      </p>
    </template>
    <NForm
      :model="answers"
      :rules="rules"
      v-else-if="current?.type === 'authChallenge'"
      ref="authForm"
      id="auth-challenge-form"
      class="form-grid"
      @submit.prevent="auth()"
    >
      <p class="span-2">{{ current.instructions }}</p>
      <NFormItem
        :label="prompt.text"
        :path="String(index)"
        v-for="(prompt, index) in current.prompts"
        :key="index"
        class="span-2"
      >
        <AppInput
          v-model="answers[index]"
          :type="prompt.echo ? 'text' : 'password'"
          autocomplete="off"
          :autofocus="index === 0"
          maxlength="16384"
        />
      </NFormItem>
    </NForm>
    <p v-if="error" class="error" role="alert">{{ error }}</p>
    <template #footer>
      <NFlex v-if="current?.type === 'hostKeyChallenge'" justify="end">
        <AppButton :disabled="busy" @click="hostKey('reject')">
          {{ t("knownHosts.actions.reject") }}
        </AppButton>
        <AppButton :disabled="busy" @click="hostKey('allowOnce')">
          {{ t("knownHosts.actions.trustOnce") }}
        </AppButton>
        <AppButton
          class="primary"
          :disabled="busy"
          @click="hostKey('trustAndSave')"
        >
          {{ t("knownHosts.actions.trustAndSave") }}
        </AppButton>
      </NFlex>
      <NFlex v-else-if="current?.type === 'authChallenge'" justify="end">
        <AppButton type="button" :disabled="busy" @click="auth(true)">
          {{ t("terminal.actions.cancelConnection") }}
        </AppButton>
        <AppButton
          id="auth-challenge-continue"
          type="submit"
          form="auth-challenge-form"
          class="primary"
          :disabled="busy"
        >
          {{ t("common.actions.continue") }}
        </AppButton>
      </NFlex>
    </template>
  </AppDialog>
</template>

<style scoped>
.fingerprint {
  display: grid;
  grid-template-columns: 90px 1fr;
  gap: 14px;
  padding: 16px;
  background: var(--input);
  border-radius: 6px;
}
.fingerprint dt {
  color: var(--muted);
}
.fingerprint dd {
  margin: 0;
  overflow-wrap: anywhere;
  font-family: monospace;
  font-size: 12px;
}
</style>
