<script setup lang="ts">
import { t } from "@/i18n";
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
import AppDialog from "@/components/ui/AppDialog.vue";
import {
  integerRange,
  requiredText,
  validateForm,
} from "@/services/forms/rules";
import { errorMessage } from "@/services/desktop/client";
import type {
  ForwardDraft,
  ForwardMode,
  ForwardProfile,
} from "@/services/desktop/types";
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
          ? t("common.format.copyName", { name: profile.name })
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
    requiredText("common.fields.name"),
    {
      max: 128,
      message: t("forwarding.validation.nameLength"),
      trigger: ["input", "blur"],
    },
  ],
  profileId: {
    required: true,
    trigger: "change",
    validator: (_, value) =>
      hostOptions.value.some((h) => h.value === value) ||
      new Error(t("hosts.selection.sshRequired")),
  },
  "rule.bindAddress": requiredText("forwarding.form.bindAddress"),
  "rule.bindPort": integerRange("forwarding.form.bindPort", 1, 65535),
  ...(draft.rule.mode === "dynamic"
    ? {}
    : {
        "rule.targetAddress": requiredText("forwarding.form.targetAddress"),
        "rule.targetPort": integerRange("forwarding.form.targetPort", 1, 65535),
      }),
}));
const description = computed(
  () =>
    ({
      local: t("forwarding.descriptions.local"),
      remote: t("forwarding.descriptions.remote"),
      dynamic: t("forwarding.descriptions.dynamic"),
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
    :title="
      draft.id ? t('forwarding.actions.edit') : t('forwarding.actions.new')
    "
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
      <NFormItem :label="t('common.fields.name')" path="name">
        <NInput
          v-model:value="draft.name"
          :maxlength="128"
          :placeholder="t('forwarding.form.namePlaceholder')"
        />
      </NFormItem>
      <NFormItem :label="t('common.connection.sshHost')" path="profileId">
        <NSelect
          v-model:value="draft.profileId"
          :options="hostOptions"
          filterable
          :placeholder="t('hosts.selection.sshPlaceholder')"
        />
      </NFormItem>
      <NAlert v-if="!hostOptions.length" type="info">
        {{ t("forwarding.form.hostHint") }}
      </NAlert>
      <NFormItem :label="t('forwarding.form.mode')" path="rule.mode">
        <NRadioGroup :value="draft.rule.mode" @update:value="changeMode">
          <NRadioButton value="local">
            {{ t("forwarding.modes.local") }}
          </NRadioButton>
          <NRadioButton value="remote">
            {{ t("forwarding.modes.remote") }}
          </NRadioButton>
          <NRadioButton value="dynamic">
            {{ t("forwarding.modes.dynamic") }}
          </NRadioButton>
        </NRadioGroup>
      </NFormItem>
      <p>
        <NText depth="3">{{ description }}</NText>
      </p>
      <NGrid :cols="2" :x-gap="16">
        <NFormItemGi
          :label="
            draft.rule.mode === 'remote'
              ? t('forwarding.form.remoteBindAddress')
              : t('forwarding.form.localBindAddress')
          "
          path="rule.bindAddress"
        >
          <NInput
            v-model:value="draft.rule.bindAddress"
            :placeholder="t('forwarding.form.loopbackPlaceholder')"
          />
        </NFormItemGi>
        <NFormItemGi
          :label="t('forwarding.form.bindPort')"
          path="rule.bindPort"
        >
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
                ? t('forwarding.form.localTargetAddress')
                : t('forwarding.form.remoteTargetAddress')
            "
            path="rule.targetAddress"
          >
            <NInput
              :value="draft.rule.targetAddress ?? ''"
              @update:value="draft.rule.targetAddress = $event"
              :placeholder="t('forwarding.form.targetPlaceholder')"
            />
          </NFormItemGi>
          <NFormItemGi
            :label="t('forwarding.form.targetPort')"
            path="rule.targetPort"
          >
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
        {{ t("forwarding.warning.publicListener") }}
      </NText>
    </NForm>
    <NAlert v-if="error" type="error">{{ error }}</NAlert>
    <template #footer>
      <NFlex justify="end">
        <NButton :disabled="busy" @click="emit('update:open', false)">
          {{ t("common.actions.cancel") }}
        </NButton>
        <NButton
          type="primary"
          :loading="busy"
          :disabled="!hostOptions.length"
          @click="save"
        >
          {{ t("common.actions.save") }}
        </NButton>
      </NFlex>
    </template>
  </AppDialog>
</template>
