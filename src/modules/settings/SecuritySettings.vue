<script setup lang="ts">
import { ref, onMounted } from "vue";
import { confirmAction } from "../../components/ui/confirmation";
import { desktop, errorMessage } from "../../services/desktop/client";
import type {
  KnownHost,
  CredentialMetadata,
  HistoryEntry,
} from "../../services/desktop/types";
const error = ref(""),
  known = ref<KnownHost[]>([]),
  credentials = ref<CredentialMetadata[]>([]),
  history = ref<HistoryEntry[]>([]);
onMounted(loadSecurity);
async function loadSecurity() {
  try {
    [known.value, credentials.value, history.value] = await Promise.all([
      desktop.listKnownHosts(),
      desktop.listCredentials(),
      desktop.listHistory(),
    ]);
  } catch (e) {
    error.value = errorMessage(e);
  }
}
async function clearHistory() {
  if (await confirmAction("清空本地连接历史？不会删除主机配置。"))
    try {
      await desktop.clearHistory();
      await loadSecurity();
    } catch (e) {
      error.value = errorMessage(e);
    }
}
async function revoke(id: string) {
  if (await confirmAction("撤销信任？下次连接需要重新核验指纹。"))
    try {
      await desktop.revokeKnownHost(id);
      await loadSecurity();
    } catch (e) {
      error.value = errorMessage(e);
    }
}
async function deleteCredential(id: string) {
  if (await confirmAction("删除系统凭据？主机引用可能需要重新填写。"))
    try {
      await desktop.deleteCredential(id);
      await loadSecurity();
    } catch (e) {
      error.value = errorMessage(e);
    }
}
</script>
<template>
  <div>
    <h3>已信任的 SSH 主机</h3>
    <p v-if="!known.length" class="muted">尚未保存主机信任。</p>
    <article v-for="host in known" :key="host.id" class="security-row">
      <div>
        <strong>{{ host.canonicalHost + ":" + host.port }}</strong
        ><small>{{ host.keyAlgorithm }} · {{ host.fingerprintSha256 }}</small>
      </div>
      <button class="danger" @click="revoke(host.id)">撤销信任</button>
    </article>
    <h3>连接历史</h3>
    <button @click="clearHistory">清空历史</button>
    <p v-if="!history.length" class="muted">尚无连接历史。</p>
    <article v-for="item in history" :key="item.id" class="security-row">
      <div>
        <strong>{{ item.displayName }}</strong
        ><small
          >{{ item.endpoint }} ·
          {{ new Date(item.startedAt).toLocaleString() }} ·
          {{ item.endReason || "进行中" }} {{ item.errorCode }}</small
        >
      </div>
    </article>
    <h3>系统凭据</h3>
    <p v-if="!credentials.length" class="muted">
      尚未保存凭据，或系统凭据库不可用。
    </p>
    <article
      v-for="credential in credentials"
      :key="credential.id"
      class="security-row"
    >
      <div>
        <strong>{{ credential.label }}</strong
        ><small>{{ credential.kind }} · {{ credential.id }}</small>
      </div>
      <button
        v-if="credential.status === 'pending'"
        @click="
          desktop
            .recoverCredential(credential.id)
            .then(loadSecurity)
            .catch((e) => (error = errorMessage(e)))
        "
      >
        恢复一致性</button
      ><button class="danger" @click="deleteCredential(credential.id)">
        删除凭据
      </button>
    </article>
  </div>
  <p v-if="error" class="error" role="alert">{{ error }}</p>
</template>
