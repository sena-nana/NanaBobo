<script setup lang="ts">
import { RouterLink } from "vue-router";
import { Card } from "../../ui";
import AccountPanel from "../account/AccountPanel.vue";
import RoomInfoPanel from "../live/RoomInfoPanel.vue";
import { useNanaSession } from "../session/useNanaSession";
import LiveTrendChart from "../stats/LiveTrendChart.vue";

const session = useNanaSession();
const { account, room, stats, authenticated } = session;
</script>

<template>
  <section class="home-page" data-agent-id="home.page">
    <span class="home-header-marker" data-agent-id="home.header" aria-hidden="true" />
    <div class="home-dashboard" data-agent-id="home.dashboard">
      <div class="home-room" data-agent-id="home.room">
        <AccountPanel v-if="!authenticated" :session="account" />
        <RoomInfoPanel v-else :session="room" @disconnect="session.disconnectRoom" />
      </div>
      <Card class="home-trend-card" data-agent-id="home.trend-card">
        <LiveTrendChart :snapshots="stats.current.value" :current-room="room.info.value" />
      </Card>
    </div>

    <section class="home-tools" data-agent-id="home.tools">
      <div class="tools-heading">
        <div>
          <p class="section-kicker">工具</p>
          <h2>实时直播工具</h2>
        </div>
        <RouterLink class="tools-link" to="/assistant">主播助手</RouterLink>
      </div>
      <RouterLink class="tool-card" to="/assistant" data-agent-id="home.tool.danmaku">
        <span class="tool-icon" aria-hidden="true">弹</span>
        <span class="tool-copy">
          <strong>弹幕助手</strong>
        </span>
        <span class="tool-arrow" aria-hidden="true">→</span>
      </RouterLink>
    </section>
  </section>
</template>

<style scoped>
.home-page { display: grid; min-height: 100%; grid-template-rows: minmax(0, 1fr) auto; gap: 14px; }
.home-header-marker { display: none; }
.home-dashboard { display: grid; min-height: 0; grid-template-columns: minmax(230px, 270px) minmax(360px, 1fr); grid-auto-rows: minmax(330px, 1fr); gap: clamp(18px, 3vw, 34px); align-items: stretch; padding: 32px 0 48px; }
.home-room { min-width: 0; text-align: center; }
.home-room :deep(.ui-card), .home-trend-card { min-height: 330px; margin-bottom: 0; border: 1px solid var(--border); background: var(--bg-elev); }
.home-room :deep(.ui-card) { width: 100%; height: 100%; }
.home-room :deep(.account-card) { display: grid; align-content: center; padding: 16px; text-align: center; }
.home-room :deep(.card-heading) { justify-content: center; flex-direction: column; margin-bottom: 12px; }
.home-room :deep(.login-block), .home-room :deep(.state-block), .home-room :deep(.qr-block) { justify-items: center; min-height: 0; text-align: center; }
.home-room :deep(.account-row) { min-height: 0; }
.home-trend-card { display: grid; min-width: 0; place-items: center; }
.home-trend-card :deep(.trend-chart) { width: 100%; }
.home-tools { min-height: 150px; padding: 14px 0 0; border-top: 1px solid var(--border-soft); }
.tools-heading { display: flex; align-items: center; justify-content: center; gap: 16px; margin-bottom: 14px; text-align: center; }
.tools-heading h2 { margin: 0; font-size: 18px; font-weight: 600; }
.tools-link { color: var(--accent-text); font-size: 13px; text-decoration: none; }
.tools-link:hover { text-decoration: underline; }
.tool-card { display: grid; max-width: 390px; margin: 0 auto; grid-template-columns: auto minmax(0, 1fr) auto; align-items: center; gap: 12px; padding: 12px 14px; border: 1px solid var(--border-soft); border-radius: 8px; color: inherit; text-decoration: none; transition: border-color .12s ease, background .12s ease; }
.tool-card:hover { border-color: var(--accent); background: var(--bg-hover); }
.tool-icon { display: grid; width: 36px; height: 36px; place-items: center; border-radius: 50%; background: var(--accent); color: white; font-weight: 700; }
.tool-copy { display: grid; min-width: 0; gap: 4px; }
.tool-copy strong { font-size: 14px; }
.tool-arrow { color: var(--text-faint); font-size: 18px; }
:global(.nana-avatar-fallback) { display: grid; place-items: center; background: var(--accent-soft); color: var(--accent-text); font-size: 24px; font-weight: 700; }
@media (max-width: 760px) {
  .home-page { grid-template-rows: auto auto; gap: 12px; }
  .home-dashboard { grid-template-columns: 1fr; grid-auto-rows: auto; gap: 16px; align-items: start; padding: 24px 0 32px; }
  .home-room :deep(.ui-card) { height: auto; }
  .home-room :deep(.ui-card), .home-trend-card { min-height: 0; }
  .home-tools { padding: 12px 0 0; }
}
</style>
