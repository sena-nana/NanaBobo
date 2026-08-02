<script setup lang="ts">
import { computed, onMounted } from "vue";
import { Button, Card } from "../../ui";
import { useAccountSession } from "./useAccountSession";

const { available, status, qr, qrState, loading, polling, error, loadStatus, startLogin, pollOnce, logout } = useAccountSession();
const accountLabel = computed(() => status.value?.account?.username ?? "未登录");
const qrHint = computed(() => {
  if (qrState.value === "scanned") return "已扫描，请在手机上确认登录。";
  if (qrState.value === "expired") return "二维码已过期，请重新生成。";
  return "请使用B站手机客户端扫描二维码。";
});

onMounted(() => { void loadStatus(); });
</script>

<template>
  <Card class="feature-card" data-agent-id="account.panel">
    <div class="card-heading">
      <div>
        <h2>B站账号</h2>
        <p class="muted">登录后可使用个人账号相关的直播工具。</p>
      </div>
      <span class="status" role="status">{{ accountLabel }}</span>
    </div>

    <div v-if="!available" class="empty-state">
      <p>请在桌面应用中管理B站账号。</p>
    </div>
    <div v-else-if="loading && !status" class="state-block" role="status">
      正在检查登录状态…
    </div>
    <div v-else-if="status?.authenticated && status.account" class="account-row">
      <img
        v-if="status.account.avatar_url"
        class="avatar"
        :src="status.account.avatar_url"
        alt=""
      >
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
      <p class="muted">使用二维码登录，登录态保存在本机安全凭据存储中。</p>
      <Button variant="primary" data-agent-id="account.login" :loading="loading" @click="startLogin">
        扫码登录
      </Button>
    </div>

    <p v-if="error" class="error" role="alert">{{ error.message }}</p>
  </Card>
</template>

<style scoped>
.feature-card { min-height: 238px; }
.card-heading, .account-row, .actions { display: flex; align-items: center; gap: 12px; }
.card-heading { justify-content: space-between; margin-bottom: 18px; }
.card-heading h2 { margin: 0; }
.card-heading p { margin: 5px 0 0; }
.status { color: var(--text-muted); font-size: 12px; }
.state-block, .empty-state, .login-block, .qr-block { display: grid; gap: 12px; align-content: center; min-height: 150px; }
.account-row { min-height: 150px; }
.account-copy { display: grid; gap: 4px; min-width: 0; flex: 1; }
.avatar { width: 42px; height: 42px; border-radius: 50%; object-fit: cover; }
.qr-block { justify-items: center; text-align: center; }
.qr-image { display: grid; width: 168px; height: 168px; padding: 6px; background: white; border-radius: 6px; }
.qr-image :deep(svg) { width: 100%; height: 100%; }
.qr-block p, .login-block p, .empty-state p, .state-block { margin: 0; }
.actions { justify-content: center; }
.error { margin: 14px 0 0; color: var(--err); }
@media (max-width: 640px) {
  .card-heading { align-items: flex-start; flex-direction: column; }
  .account-row { align-items: flex-start; flex-wrap: wrap; }
}
</style>
