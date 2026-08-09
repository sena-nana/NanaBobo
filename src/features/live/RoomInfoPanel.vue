<script setup lang="ts">
import { computed } from "vue";
import { Button, Card, Input } from "../../ui";
import type { RoomInfoSession } from "./useRoomInfo";

const props = defineProps<{ session: RoomInfoSession }>();
const emit = defineEmits<{ disconnect: [] }>();
const { roomId, info, loading, error, query } = props.session;

const ownerName = computed(() => info.value?.owner_name || `UID ${info.value?.owner_id ?? ""}`);
const ownerInitial = computed(() => ownerName.value.slice(0, 1).toUpperCase());
const liveLabel = computed(() => {
  if (!info.value) return "";
  if (info.value.live_status === "live") return "直播中";
  if (info.value.live_status === "round") return "轮播中";
  return "未开播";
});

function submitQuery() {
  void query();
}
</script>

<template>
  <Card class="room-panel" data-agent-id="room.panel">
    <div class="panel-heading">
      <div>
        <p class="nana-eyebrow">直播间</p>
        <h2>当前连接</h2>
      </div>
      <span v-if="info" class="connection-status" role="status">
        已连接
      </span>
    </div>

    <div v-if="info" class="connected-room" data-agent-id="room.info">
      <img
        v-if="info.owner_avatar_url"
        class="owner-avatar"
        :src="info.owner_avatar_url"
        alt=""
        referrerpolicy="no-referrer"
      >
      <span v-else class="owner-avatar nana-avatar-fallback nana-room-fallback" aria-hidden="true">{{ ownerInitial }}</span>
      <strong class="owner-name">{{ ownerName }}</strong>
      <dl>
        <div><dt>状态</dt><dd>{{ liveLabel }}</dd></div>
        <div><dt>在线</dt><dd>{{ info.viewer_count.toLocaleString() }}</dd></div>
        <div><dt>关注</dt><dd>{{ info.follower_count?.toLocaleString() ?? "暂无" }}</dd></div>
        <div><dt>房间号</dt><dd>{{ info.room_id }}</dd></div>
      </dl>
      <Button variant="text" data-agent-id="room.disconnect" @click="emit('disconnect')">
        切换直播间
      </Button>
    </div>

    <form v-else class="connect-form" data-agent-id="room.connect-form" @submit.prevent="submitQuery">
      <div>
        <h3>连接直播间</h3>
      </div>
      <Input
        :model-value="roomId"
        inputmode="numeric"
        placeholder="输入直播间号"
        aria-label="直播间号"
        data-agent-id="room.input"
        @update:model-value="roomId = String($event)"
      />
      <Button variant="primary" type="submit" data-agent-id="room.query" :loading="loading">
        连接直播间
      </Button>
    </form>

    <p v-if="error" class="error" role="alert">{{ error.message }}</p>
  </Card>
</template>

<style scoped>
.room-panel { min-width: 220px; height: 100%; }
.panel-heading { display: flex; align-items: flex-start; justify-content: space-between; gap: 12px; margin-bottom: 22px; }
.panel-heading h2, .connect-form h3 { margin: 0; font-size: 18px; }
.connection-status { display: inline-flex; align-items: center; gap: 6px; color: var(--ok); font-size: 12px; white-space: nowrap; }
.connected-room { display: grid; justify-items: center; gap: 10px; text-align: center; }
.owner-avatar { width: 84px; height: 84px; border-radius: 50%; object-fit: cover; background: var(--bg-subtle); }
dl { width: 100%; display: grid; gap: 8px; margin: 12px 0 8px; text-align: left; }
dl div { display: flex; justify-content: space-between; gap: 12px; font-size: 12px; }
dt { color: var(--text-muted); }
dd { margin: 0; color: var(--text); }
.connect-form { display: grid; gap: 14px; align-content: center; min-height: 300px; }
.connect-form :deep(input) { width: 100%; }
.error { margin: 16px 0 0; color: var(--err); line-height: 1.45; }
@media (max-width: 760px) {
  .room-panel { height: auto; }
  .connected-room { justify-items: start; text-align: left; }
  .connect-form { min-height: 0; }
}
</style>
