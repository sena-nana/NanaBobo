<script setup lang="ts">
import { computed } from "vue";
import type { RoomInfo, StatsSnapshot } from "../../contracts/bilibili";

const props = defineProps<{
  snapshots: StatsSnapshot[];
  currentRoom?: RoomInfo | null;
}>();
const width = 560;
const height = 250;
const plot = { left: 44, right: 18, top: 0, bottom: 34 };

const points = computed<StatsSnapshot[]>(() => {
  const stored = props.snapshots.slice(-5);
  if (stored.length > 0 || !props.currentRoom) return stored;
  return [{
    room_id: props.currentRoom.room_id,
    captured_at: props.currentRoom.fetched_at,
    viewer_count: props.currentRoom.viewer_count,
    follower_count: props.currentRoom.follower_count,
    live_status: props.currentRoom.live_status,
  }];
});
const maxValue = computed(() => {
  const values = points.value.flatMap((point) => [point.viewer_count, point.follower_count ?? 0]);
  return Math.max(100, ...values, 0);
});

function x(index: number) {
  const span = width - plot.left - plot.right;
  return points.value.length <= 1 ? plot.left + span / 2 : plot.left + (span * index) / (points.value.length - 1);
}

function y(value: number) {
  const span = height - plot.top - plot.bottom;
  return plot.top + span - (value / maxValue.value) * span;
}

function line(field: "viewer_count" | "follower_count") {
  return points.value
    .map((point, index) => `${x(index)},${y(point[field] ?? 0)}`)
    .join(" ");
}

function label(timestamp: number) {
  return new Date(timestamp * 1000).toLocaleDateString("zh-CN", { month: "2-digit", day: "2-digit" });
}

const gridLines = computed(() => [0, 0.25, 0.5, 0.75, 1].map((ratio) => ({
  y: y(maxValue.value * ratio),
  label: Math.round(maxValue.value * ratio).toLocaleString(),
})));
</script>

<template>
  <section class="trend-chart" data-agent-id="home.trend">
    <div class="trend-heading">
      <div class="legend" aria-label="图例">
        <span><i class="legend-dot viewer" aria-hidden="true" />在线人数</span>
        <span><i class="legend-dot follower" aria-hidden="true" />关注数</span>
      </div>
    </div>

    <div class="trend-plot">
      <svg class="trend-svg" :viewBox="`0 0 ${width} ${height}`" role="img" aria-label="直播间在线人数和关注数趋势图">
      <g class="grid-lines">
        <line v-for="grid in gridLines" :key="grid.y" :x1="plot.left" :x2="width - plot.right" :y1="grid.y" :y2="grid.y" />
        <text v-for="grid in gridLines" :key="`label-${grid.y}`" :x="plot.left - 10" :y="grid.y + 4" text-anchor="end">{{ grid.label }}</text>
      </g>
      <polyline v-if="points.length > 0" class="trend-line viewer" :points="line('viewer_count')" />
      <polyline v-if="points.some((point) => point.follower_count !== null)" class="trend-line follower" :points="line('follower_count')" />
      <g class="axis-labels">
        <text v-for="(point, index) in points" :key="point.captured_at" :x="x(index)" :y="height - 8" text-anchor="middle">{{ label(point.captured_at) }}</text>
      </g>
      <g class="points viewer">
        <circle v-for="(point, index) in points" :key="`viewer-${point.captured_at}`" :cx="x(index)" :cy="y(point.viewer_count)" r="4" />
      </g>
      <g v-if="points.some((point) => point.follower_count !== null)" class="points follower">
        <circle v-for="(point, index) in points" :key="`follower-${point.captured_at}`" :cx="x(index)" :cy="y(point.follower_count ?? 0)" r="4" />
      </g>
      </svg>
      <p v-if="points.length === 0" class="trend-empty" role="status">连接直播间后开始记录数据</p>
    </div>
  </section>
</template>

<style scoped>
.trend-chart { min-width: 0; padding: 4px 0 0; }
.trend-heading { display: flex; align-items: center; justify-content: center; gap: 18px; }
.legend { display: flex; flex-wrap: wrap; justify-content: center; gap: 14px; color: var(--text-muted); font-size: 12px; }
.legend span { display: inline-flex; align-items: center; gap: 6px; white-space: nowrap; }
.legend-dot { width: 9px; height: 9px; border-radius: 50%; }
.legend-dot.viewer { background: var(--accent); }
.legend-dot.follower { background: var(--ok); }
.trend-plot { position: relative; min-height: 250px; }
.trend-empty { position: absolute; inset: 0; display: grid; margin: 0; place-items: center; color: var(--text-faint); font-size: 13px; pointer-events: none; }
.trend-svg { display: block; width: 100%; height: 250px; margin-top: 0; overflow: visible; }
.grid-lines line { stroke: var(--border-soft); stroke-width: 1; }
.grid-lines text, .axis-labels text { fill: var(--text-muted); font-size: 11px; }
.trend-line { fill: none; stroke-width: 2.5; stroke-linejoin: round; stroke-linecap: round; }
.trend-line.viewer { stroke: var(--accent); }
.trend-line.follower { stroke: var(--ok); }
.points circle { fill: var(--bg-elev); stroke-width: 2; }
.points.viewer circle { stroke: var(--accent); }
.points.follower circle { stroke: var(--ok); }
@media (max-width: 760px) {
  .trend-heading { gap: 8px; }
  .trend-svg { height: 210px; }
}
</style>
