<script setup lang="ts">
import { computed } from "vue";
import AccountPanel from "../account/AccountPanel.vue";
import RoomInfoPanel from "../live/RoomInfoPanel.vue";
import { useNanaSession } from "../session/useNanaSession";
import { Button, Card } from "../../ui";

const session = useNanaSession();
const { account, room, danmaku, authenticated } = session;
const statusLabel = computed(() => {
  switch (danmaku.status.value.state) {
    case "connected": return "已连接";
    case "connecting": return "连接中";
    case "reconnecting": return "重连中";
    case "error": return "连接异常";
    case "stopped": return "已停止";
    default: return "未连接";
  }
});
const canStop = computed(() => Boolean(danmaku.status.value.connection_id));
</script>

<template>
  <section class="assistant-page" data-agent-id="assistant.page">
    <template v-if="!authenticated">
      <div class="assistant-auth"><AccountPanel :session="account" /></div>
    </template>
    <template v-else>
      <header class="page-header assistant-header" data-agent-id="assistant.header">
        <div>
          <p class="section-kicker">主播助手</p>
          <h1>弹幕助手</h1>
          <p>实时查看当前直播间收到的弹幕。</p>
        </div>
      </header>
      <div class="assistant-layout">
        <RoomInfoPanel :session="room" @disconnect="session.disconnectRoom" />
        <Card class="danmaku-card" data-agent-id="assistant.danmaku">
          <div class="danmaku-heading">
            <div>
              <p class="section-kicker">实时消息</p>
              <h2>{{ room.info.value?.title || "连接直播间后开始监控" }}</h2>
            </div>
            <div class="danmaku-actions">
              <span class="connection-state" :data-state="danmaku.status.value.state" role="status">{{ statusLabel }}</span>
              <Button v-if="canStop" variant="text" data-agent-id="assistant.danmaku.stop" :loading="danmaku.loading.value" @click="danmaku.stop">
                停止
              </Button>
              <Button v-else variant="primary" data-agent-id="assistant.danmaku.start" :loading="danmaku.loading.value" :disabled="!room.info.value" @click="danmaku.start">
                开始监控
              </Button>
            </div>
          </div>

          <div v-if="!danmaku.available" class="danmaku-empty" role="status">
            请在桌面应用中使用弹幕助手。
          </div>
          <div v-else-if="!room.info.value" class="danmaku-empty" role="status">
            先在左侧连接一个直播间。
          </div>
          <div v-else-if="danmaku.messages.value.length === 0" class="danmaku-empty" role="status">
            {{ canStop ? "等待新的弹幕…" : "点击开始监控，接收实时弹幕。" }}
          </div>
          <ol v-else class="danmaku-list" data-agent-id="assistant.danmaku.list" aria-label="实时弹幕">
            <li v-for="(message, index) in danmaku.messages.value" :key="`${message.sent_at}-${index}`">
              <time>{{ new Date(message.sent_at * 1000).toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit" }) }}</time>
              <strong>{{ message.sender_name }}</strong>
              <span>{{ message.text }}</span>
            </li>
          </ol>
          <p v-if="danmaku.error.value" class="error" role="alert">{{ danmaku.error.value.message }}</p>
        </Card>
      </div>
    </template>
  </section>
</template>

<style scoped>
.assistant-page { display: grid; min-height: 100%; grid-template-rows: auto minmax(0, 1fr); gap: 18px; }
.assistant-auth { display: grid; min-height: 0; place-items: center; }
.assistant-header h1 { margin: 0; font-size: 22px; }
.assistant-header p:last-child { margin: 6px 0 0; color: var(--text-muted); }
.assistant-layout { display: grid; min-height: 0; grid-template-columns: minmax(220px, 270px) minmax(0, 1fr); gap: 18px; }
.danmaku-card { display: flex; min-width: 0; min-height: 0; height: 100%; flex-direction: column; }
.danmaku-heading { display: flex; align-items: center; justify-content: space-between; gap: 16px; padding-bottom: 16px; border-bottom: 1px solid var(--border-soft); }
.danmaku-heading h2 { max-width: 520px; margin: 0; overflow: hidden; font-size: 18px; text-overflow: ellipsis; white-space: nowrap; }
.danmaku-actions { display: flex; align-items: center; gap: 12px; }
.connection-state { color: var(--text-muted); font-size: 12px; white-space: nowrap; }
.connection-state[data-state="connected"] { color: var(--ok); }
.connection-state[data-state="error"] { color: var(--err); }
.danmaku-empty { display: grid; flex: 1; place-items: center; min-height: 220px; color: var(--text-faint); font-size: 13px; }
.danmaku-list { display: flex; flex: 1; min-height: 0; margin: 0; padding: 10px 0 0; overflow: auto; flex-direction: column; gap: 1px; list-style: none; }
.danmaku-list li { display: grid; grid-template-columns: 56px minmax(80px, 150px) minmax(0, 1fr); gap: 10px; padding: 8px 0; border-bottom: 1px solid var(--border-soft); font-size: 13px; }
.danmaku-list time { color: var(--text-faint); font-variant-numeric: tabular-nums; }
.danmaku-list strong { overflow: hidden; color: var(--accent-text); text-overflow: ellipsis; white-space: nowrap; }
.danmaku-list span { overflow-wrap: anywhere; }
.error { margin: 12px 0 0; color: var(--err); }
@media (max-width: 760px) {
  .assistant-page { gap: 14px; }
  .assistant-layout { grid-template-columns: 1fr; }
  .danmaku-card { min-height: 500px; }
  .danmaku-heading { align-items: flex-start; flex-direction: column; }
}
</style>
