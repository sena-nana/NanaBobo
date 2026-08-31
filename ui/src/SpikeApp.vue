<script setup lang="ts">
import { onMounted, ref } from "vue";
import NanaButton from "@nanaui/nanavue-components/NanaButton";
import NanaCard from "@nanaui/nanavue-components/NanaCard";
import NanaInput from "@nanaui/nanavue-components/NanaInput";
import { nanaHost } from "./nanaHost";

const host = nanaHost();

const bridgeReport = ref("桥接探测中…");
const eventReport = ref("等待 Rust 事件…");
const storageReport = ref("localStorage 探测中…");
const visits = ref(0);
const count = ref(0);
const nickname = ref("娜娜");
const dark = ref(document.documentElement.dataset.theme === "dark");

onMounted(async () => {
  const syncEcho = host.call("spike_echo", ["同步桥"]);
  const asyncEcho = (await host.invoke("spike_echo", ["异步载荷"])) as string;
  bridgeReport.value = `${String(syncEcho)} / ${String(asyncEcho)}`;

  host.on("spike:pong", (payload) => {
    eventReport.value = `收到 Rust 事件:${String(payload)}`;
  });
  await host.invoke("spike_ping_event");

  const KEY = "spike.visits";
  const previous = Number(localStorage.getItem(KEY) ?? "0");
  visits.value = previous + 1;
  localStorage.setItem(KEY, String(visits.value));
  storageReport.value = `第 ${visits.value} 次启动(键 spike.visits 已写穿宿主文件)`;
});

function toggleTheme() {
  dark.value = !dark.value;
  document.documentElement.dataset.theme = dark.value ? "dark" : "light";
}
</script>

<template>
  <div class="spike-page">
    <header class="page-header">
      <p class="section-kicker">MIGRATION SPIKE</p>
      <h1>NanaBobo 迁移验证页</h1>
      <p class="page-lede">验证样式子集、组件桥、宿主桥与持久化,为逐页迁移提供依据。</p>
    </header>

    <section class="probe-grid">
      <NanaCard class="probe-card" title="宿主桥">
        <p class="probe-line">{{ bridgeReport }}</p>
        <p class="probe-line">{{ eventReport }}</p>
      </NanaCard>

      <NanaCard class="probe-card" title="持久化">
        <p class="probe-line">{{ storageReport }}</p>
        <p class="probe-line">重启本应用后数字应继续递增。</p>
      </NanaCard>

      <NanaCard class="probe-card" title="控件与样式">
        <div class="control-row">
          <NanaButton kind="primary" :label="`点击 ${count} 次`" @press="count++" />
          <NanaButton :label="dark ? '切到浅色' : '切到深色'" @press="toggleTheme" />
        </div>
        <NanaInput v-model="nickname" placeholder="输入昵称" class="nickname-input" />
        <p class="probe-line greeting">你好,{{ nickname }},中文渲染与输入双向绑定正常。</p>
      </NanaCard>

      <NanaCard class="probe-card" title="深层选择器">
        <p class="probe-line">
          scoped 样式 + <code>:deep()</code> + oklch 变量:下面每个色块由 scoped 规则绘制。
        </p>
        <div class="swatch-row">
          <span class="swatch swatch-accent"></span>
          <span class="swatch swatch-surface"></span>
          <span class="swatch swatch-border"></span>
          <span class="swatch swatch-deep"></span>
        </div>
      </NanaCard>
    </section>
  </div>
</template>

<style scoped>
.spike-page {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
  background: var(--spike-surface);
  color: var(--spike-text);
}

.probe-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 16px;
  padding: 0 24px 32px;
}

.probe-card {
  border-radius: 12px;
  padding: 4px;
}

.probe-line {
  margin: 6px 0;
  font-size: 13px;
  line-height: 1.6;
}

.control-row {
  display: flex;
  gap: 12px;
  align-items: center;
  margin: 10px 0;
}

.nickname-input {
  margin: 8px 0;
}

.greeting {
  color: var(--spike-accent);
}

.swatch-row {
  display: flex;
  gap: 10px;
  margin-top: 10px;
}

.swatch {
  width: 48px;
  height: 24px;
  border-radius: 6px;
}

.swatch-accent {
  background: var(--spike-accent);
}

.swatch-surface {
  background: var(--spike-surface);
  border: 1px solid var(--spike-border);
}

.swatch-border {
  background: var(--spike-border);
}

.swatch-deep {
  background: oklch(0.7 0.15 150);
}
</style>
