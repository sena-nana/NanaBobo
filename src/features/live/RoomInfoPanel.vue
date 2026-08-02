<script setup lang="ts">
import { computed } from "vue";
import { Button, Card, Input } from "../../ui";
import { useRoomInfo } from "./useRoomInfo";

const { roomId, info, loading, error, query } = useRoomInfo();
const liveLabel = computed(() => {
  if (!info.value) return "";
  if (info.value.live_status === "live") return "直播中";
  if (info.value.live_status === "round") return "轮播中";
  return "未开播";
});
</script>

<template>
  <Card class="feature-card" data-agent-id="room.panel">
    <div class="card-heading">
      <div>
        <h2>直播间信息</h2>
        <p class="muted">输入房间号，查看当前公开直播信息。</p>
      </div>
    </div>

    <form class="room-form" @submit.prevent="query">
      <Input
        :model-value="roomId"
        inputmode="numeric"
        placeholder="输入直播间号"
        aria-label="直播间号"
        data-agent-id="room.input"
        @update:model-value="roomId = String($event)"
      />
      <Button variant="primary" type="submit" data-agent-id="room.query" :loading="loading">
        查询
      </Button>
    </form>

    <div v-if="info" class="room-result" data-agent-id="room.info">
      <img v-if="info.cover_url" class="cover" :src="info.cover_url" alt="">
      <div class="room-copy">
        <div class="result-heading">
          <strong>{{ info.title }}</strong>
          <span class="status">{{ liveLabel }}</span>
        </div>
        <dl>
          <div><dt>房间号</dt><dd>{{ info.room_id }}</dd></div>
          <div><dt>主播</dt><dd>{{ info.owner_name || `UID ${info.owner_id}` }}</dd></div>
          <div><dt>在线人数</dt><dd>{{ info.viewer_count.toLocaleString() }}</dd></div>
        </dl>
      </div>
    </div>
    <p v-else class="muted empty-copy">查询结果会显示在这里。</p>
    <p v-if="error" class="error" role="alert">{{ error.message }}</p>
  </Card>
</template>

<style scoped>
.feature-card { min-height: 238px; }
.card-heading { margin-bottom: 18px; }
.card-heading h2 { margin: 0; }
.card-heading p { margin: 5px 0 0; }
.room-form { display: flex; align-items: center; gap: 8px; }
.room-form :deep(input) { min-width: 0; flex: 1; }
.room-result { display: flex; gap: 12px; margin-top: 18px; min-width: 0; }
.cover { width: 112px; height: 72px; flex: none; border-radius: 6px; object-fit: cover; background: var(--bg-subtle); }
.room-copy { min-width: 0; flex: 1; }
.result-heading { display: flex; align-items: baseline; gap: 8px; }
.result-heading strong { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.status { color: var(--accent-text); font-size: 12px; white-space: nowrap; }
dl { display: grid; gap: 5px; margin: 10px 0 0; }
dl div { display: flex; gap: 10px; font-size: 12px; }
dt { width: 52px; color: var(--text-muted); }
dd { margin: 0; color: var(--text); }
.empty-copy { margin: 18px 0 0; }
.error { margin: 14px 0 0; color: var(--err); }
@media (max-width: 640px) { .room-form { align-items: stretch; flex-direction: column; } .room-result { align-items: flex-start; } }
</style>
