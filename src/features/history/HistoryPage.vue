<script setup lang="ts">
import { computed } from "vue";
import AccountPanel from "../account/AccountPanel.vue";
import RoomInfoPanel from "../live/RoomInfoPanel.vue";
import { useNanaSession } from "../session/useNanaSession";
import { Button, Card } from "../../ui";

const session = useNanaSession();
const { account, room, stats, authenticated } = session;
const history = computed(() => [...stats.current.value].reverse());

function formatTime(timestamp: number) {
  return new Date(timestamp * 1000).toLocaleString("zh-CN", { month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit" });
}
</script>

<template>
  <section class="history-page" data-agent-id="history.page">
    <template v-if="!authenticated">
      <div class="history-auth"><AccountPanel :session="account" /></div>
    </template>
    <template v-else>
      <header class="page-header history-header">
        <div>
          <p class="section-kicker">历史记录</p>
          <h1>数据快照</h1>
          <p>查看当前直播间保存的历史采集结果。</p>
        </div>
        <Button v-if="room.info.value && history.length" variant="text" data-agent-id="history.clear" @click="stats.clearRoomHistory()">
          清理当前记录
        </Button>
      </header>
      <div class="history-layout">
        <RoomInfoPanel :session="room" @disconnect="session.disconnectRoom" />
        <Card class="history-card" data-agent-id="history.list">
          <div v-if="!room.info.value" class="history-empty" role="status">先连接一个直播间后查看历史记录。</div>
          <div v-else-if="history.length === 0" class="history-empty" role="status">当前直播间暂无历史快照。</div>
          <ol v-else class="history-list">
            <li v-for="snapshot in history" :key="`${snapshot.room_id}-${snapshot.captured_at}`">
              <time>{{ formatTime(snapshot.captured_at) }}</time>
              <span><strong>{{ snapshot.viewer_count.toLocaleString() }}</strong> 在线</span>
              <span><strong>{{ snapshot.follower_count?.toLocaleString() ?? "暂无" }}</strong> 关注</span>
              <span>{{ snapshot.live_status === "live" ? "直播中" : snapshot.live_status === "round" ? "轮播中" : "未开播" }}</span>
            </li>
          </ol>
        </Card>
      </div>
    </template>
  </section>
</template>

<style scoped>
.history-page { display: grid; min-height: 100%; grid-template-rows: auto minmax(0, 1fr); gap: 18px; }
.history-auth { display: grid; min-height: 0; place-items: center; }
.history-header { display: flex; align-items: flex-start; justify-content: space-between; gap: 16px; }
.history-header h1 { margin: 0; font-size: 22px; }
.history-header p:last-child { margin: 6px 0 0; color: var(--text-muted); }
.history-layout { display: grid; min-height: 0; grid-template-columns: minmax(220px, 270px) minmax(0, 1fr); gap: 18px; }
.history-card { min-width: 0; min-height: 0; height: 100%; overflow: hidden; }
.history-empty { display: grid; height: 100%; min-height: 260px; place-items: center; color: var(--text-faint); font-size: 13px; }
.history-list { height: 100%; margin: 0; padding: 0; overflow: auto; list-style: none; }
.history-list li { display: grid; grid-template-columns: 1.2fr .8fr .8fr .6fr; gap: 12px; padding: 13px 0; border-bottom: 1px solid var(--border-soft); color: var(--text-muted); font-size: 13px; }
.history-list li:last-child { border-bottom: 0; }
.history-list time { color: var(--text-faint); font-variant-numeric: tabular-nums; }
.history-list strong { color: var(--text); font-variant-numeric: tabular-nums; }
@media (max-width: 760px) {
  .history-page { gap: 14px; }
  .history-header { align-items: flex-start; flex-direction: column; }
  .history-layout { grid-template-columns: 1fr; }
  .history-card { min-height: 430px; }
  .history-list li { grid-template-columns: 1fr 1fr; }
}
</style>
