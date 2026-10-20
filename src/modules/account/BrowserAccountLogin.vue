<script setup lang="ts">
import { NAlert, NButton, NCollapse, NCollapseItem } from "naive-ui";
import { IconBrandGithub, IconBrandGoogle, IconWorld } from "@tabler/icons-vue";
import { formatDate, t } from "@/i18n";
import type {
  AccountOAuthFlow,
  AccountProvider,
  AccountProviderId,
} from "@/services/desktop/types";
import AccountOAuthConfirmation from "./AccountOAuthConfirmation.vue";

defineProps<{
  providers: AccountProvider[];
  flow: AccountOAuthFlow | null;
  blocked: boolean;
  working: boolean;
  providerError: string;
}>();
defineEmits<{
  start: [provider: AccountProviderId];
  cancel: [];
  retry: [];
}>();
const providerName = (provider: AccountProviderId) =>
  provider === "github"
    ? "GitHub"
    : provider === "google"
      ? "Google"
      : "termeta.io";
</script>

<template>
  <div class="browser-login">
    <p class="muted">{{ t("account.browser.description") }}</p>
    <NAlert v-if="providerError" type="warning">{{ providerError }}</NAlert>
    <div class="login-actions">
      <NButton
        v-for="provider in providers"
        :key="provider.id"
        :disabled="blocked"
        @click="$emit('start', provider.id)"
      >
        <template #icon>
          <IconBrandGithub
            v-if="provider.id === 'github'"
            :size="18"
            aria-hidden="true"
          />
          <IconBrandGoogle
            v-else-if="provider.id === 'google'"
            :size="18"
            aria-hidden="true"
          />
          <IconWorld v-else :size="18" aria-hidden="true" />
        </template>
        {{ t("account.actions.loginProvider", { provider: provider.name }) }}
      </NButton>
      <NButton v-if="providerError" :disabled="blocked" @click="$emit('retry')">
        {{ t("account.actions.retryProviders") }}
      </NButton>
    </div>
    <NAlert v-if="flow" type="info" role="status">
      <p>
        {{
          t("account.oauth.waiting", { provider: providerName(flow.provider) })
        }}
      </p>
      <p>
        {{ t("account.oauth.deadline", { date: formatDate(flow.expiresAt) }) }}
      </p>
      <p>{{ t("account.oauth.returnHint") }}</p>
      <NCollapse :key="flow.flowId" display-directive="if">
        <NCollapseItem
          name="confirmation"
          :title="t('account.oauth.manualFallback')"
        >
          <AccountOAuthConfirmation
            :flow-id="flow.flowId"
            :disabled="working"
          />
        </NCollapseItem>
      </NCollapse>
      <NButton :disabled="working" @click="$emit('cancel')">
        {{ t("account.actions.cancelLogin") }}
      </NButton>
    </NAlert>
  </div>
</template>

<style scoped>
.browser-login {
  display: flex;
  flex-direction: column;
  gap: 14px;
}
p {
  margin: 0;
}
.login-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}
</style>
