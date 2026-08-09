<script setup lang="ts">
import { computed } from "vue";
import { Button, Card } from "../../ui";
import type { AccountSession } from "./useAccountSession";

const props = defineProps<{ session: AccountSession }>();

const { available, status, qr, qrState, loading, polling, error, startLogin, pollOnce, logout } = props.session;
const accountInitial = computed(() => (status.value?.account?.username ?? "B").slice(0, 1).toUpperCase());
const accountLabel = computed(() => status.value?.account?.username ?? "未登录");
const qrHint = computed(() => {
  if (qrState.value === "scanned") return "已扫描，请在手机上确认登录。";
  if (qrState.value === "expired") return "二维码已过期，请重新生成。";
  return "请使用B站手机客户端扫描二维码。";
});
</script>

<template>
  <Card class="account-card" data-agent-id="account.panel">
    <div class="card-heading">
      <div>
        <p class="nana-eyebrow">B站账号</p>
        <h2>登录后连接直播间</h2>
      </div>
      <span class="status" role="status">{{ accountLabel }}</span>
    </div>

    <div v-if="!available" class="state-block">
      <p>请在桌面应用中扫码登录B站账号。</p>
    </div>
    <div v-else-if="loading && !status" class="state-block" role="status">
      正在检查登录状态…
    </div>
    <div v-else-if="status?.authenticated && status.account" class="account-row">
      <img
        v-if="status.account.avatar_url"
        class="account-avatar"
        :src="status.account.avatar_url"
        alt=""
        referrerpolicy="no-referrer"
      >
      <span v-else class="account-avatar nana-avatar-fallback nana-account-fallback" aria-hidden="true">{{ accountInitial }}</span>
      <div class="account-copy">
        <strong>{{ status.account.username }}</strong>
        <span class="muted">UID {{ status.account.mid }}</span>
      </div>
      <Button variant="text" data-agent-id="account.logout" :loading="loading" @click="logout">
        退出登录
      </Button>
    </div>
    <div v-else-if="qr" class="qr-block" data-agent-id="account.qr">
      <div class="qr-image" aria-label="B站登录二维码" v-html="qr.svg" />
      <p class="muted" role="status">{{ qrHint }}</p>
      <div class="actions">
        <Button :loading="polling" data-agent-id="account.check-login" @click="pollOnce">
          检查登录状态
        </Button>
        <Button variant="text" data-agent-id="account.refresh-qr" @click="startLogin">
          刷新二维码
        </Button>
      </div>
    </div>
    <div v-else class="login-block">
      <Button variant="primary" data-agent-id="account.login" :loading="loading" @click="startLogin">
        扫码登录
      </Button>
    </div>

    <p v-if="error" class="error" role="alert">{{ error.message }}</p>
  </Card>
</template>

<style scoped>
.account-card { width: min(520px, 100%); min-height: 330px; }
.card-heading, .account-row, .actions { display: flex; align-items: center; gap: 12px; }
.card-heading { justify-content: space-between; margin-bottom: 18px; }
.card-heading h2 { margin: 0; font-size: 20px; }
.state-block, .login-block, .qr-block { display: grid; gap: 12px; align-content: center; min-height: 220px; }
.account-row { min-height: 220px; }
.account-copy { display: grid; gap: 4px; min-width: 0; flex: 1; }
.account-avatar { width: 48px; height: 48px; flex: none; border-radius: 50%; object-fit: cover; }
.qr-block { justify-items: center; text-align: center; }
.qr-image { display: grid; width: 192px; height: 192px; padding: 6px; background: white; border-radius: 8px; }
.qr-image :deep(svg) { width: 100%; height: 100%; }
.actions { justify-content: center; flex-wrap: wrap; }
.error { margin: 14px 0 0; color: var(--err); }
@media (max-width: 640px) {
  .card-heading { align-items: flex-start; flex-direction: column; }
  .account-row { align-items: flex-start; flex-wrap: wrap; }
}
</style>
