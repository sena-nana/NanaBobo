<script setup lang="ts">
import { computed } from "vue";
import AccountPanel from "../account/AccountPanel.vue";
import RoomInfoPanel from "../live/RoomInfoPanel.vue";
import { useNanaSession } from "../session/useNanaSession";
import LiveTrendChart from "./LiveTrendChart.vue";
import { Card } from "../../ui";

const session = useNanaSession();
const { account, room, stats, authenticated } = session;
const latest = computed(() => stats.current.value[stats.current.value.length - 1] ?? null);
</script>

<template>
  <section class="stats-page" data-agent-id="stats.page">
    <template v-if="!authenticated">
      <div class="stats-auth"><AccountPanel :session="account" /></div>
    </template>
    <template v-else>
      <header class="page-header stats-header">
        <div>
          <p class="section-kicker">数据统计</p>
          <h1>直播数据</h1>
          <p>记录当前直播间的在线人数、关注数和直播状态。</p>
        </div>
      </header>
      <div class="stats-layout">
        <RoomInfoPanel :session="room" @disconnect="session.disconnectRoom" />
        <Card class="stats-card" data-agent-id="stats.chart">
          <LiveTrendChart :snapshots="stats.current.value" :current-room="room.info.value" />
          <div class="stats-summary" data-agent-id="stats.summary">
            <div><span>当前在线</span><strong>{{ latest?.viewer_count?.toLocaleString() ?? "暂无" }}</strong></div>
            <div><span>关注数</span><strong>{{ latest?.follower_count?.toLocaleString() ?? "暂无" }}</strong></div>
            <div><span>采集点</span><strong>{{ stats.current.value.length }}</strong></div>
          </div>
        </Card>
      </div>
    </template>
  </section>
</template>

<style scoped>
.stats-page { display: grid; min-height: 100%; grid-template-rows: auto minmax(0, 1fr); gap: 18px; }
.stats-auth { display: grid; min-height: 0; place-items: center; }
.stats-header h1 { margin: 0; font-size: 22px; }
.stats-header p:last-child { margin: 6px 0 0; color: var(--text-muted); }
.stats-layout { display: grid; min-height: 0; grid-template-columns: minmax(220px, 270px) minmax(0, 1fr); gap: 18px; }
.stats-card { display: flex; min-width: 0; min-height: 0; height: 100%; flex-direction: column; }
.stats-summary { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 1px; margin-top: auto; border-top: 1px solid var(--border-soft); background: var(--border-soft); }
.stats-summary div { display: grid; gap: 6px; min-height: 76px; padding: 14px; background: var(--bg-elev); }
.stats-summary span { color: var(--text-muted); font-size: 12px; }
.stats-summary strong { font-size: 18px; font-variant-numeric: tabular-nums; }
@media (max-width: 760px) {
  .stats-page { gap: 14px; }
  .stats-layout { grid-template-columns: 1fr; }
  .stats-card { min-height: 470px; }
}
</style>
