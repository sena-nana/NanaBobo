<script setup lang="ts">
import { computed } from "vue";
import { useRoute, useRouter } from "vue-router";
import NanaAppShell from "@nanaui/nanavue-components/NanaAppShell";
import NanaSidebarFrame from "@nanaui/nanavue-components/NanaSidebarFrame";
import NanaSidebarNav from "@nanaui/nanavue-components/NanaSidebarNav";
import NanaSidebarFooter from "@nanaui/nanavue-components/NanaSidebarFooter";
import NanaSettingsPage from "@nanaui/nanavue-components/NanaSettingsPage";
import HomePage from "../../features/home/HomePage.vue";
import DanmakuAssistantPage from "../../features/danmaku/DanmakuAssistantPage.vue";
import StatsPage from "../../features/stats/StatsPage.vue";
import HistoryPage from "../../features/history/HistoryPage.vue";
import NanaSessionProvider from "../../features/session/NanaSessionProvider.vue";

const route = useRoute();
const router = useRouter();

const navItems = [
  { key: "/", label: "首页", agentId: "sidebar.nav.home" },
  { key: "/assistant", label: "主播助手", agentId: "sidebar.nav.assistant" },
  { key: "/stats", label: "数据统计", agentId: "sidebar.nav.stats" },
  { key: "/history", label: "历史记录", agentId: "sidebar.nav.history" },
];

const activeKey = computed(() =>
  navItems.find((item) => item.key === route.path)?.key ?? "",
);

function onSelect(item: { key?: string }) {
  if (item.key) void router.push(item.key);
}
</script>

<template>
  <NanaAppShell title="Nana播播工具箱" class="app-shell">    <div class="app-layout">
      <NanaSidebarFrame class="app-sidebar" aria-label="主导航">
        <NanaSidebarNav
          :items="navItems"
          :active-key="activeKey"
          data-agent-id="sidebar.nav"
          @select="onSelect"
        />
        <NanaSidebarFooter>
          <button
            type="button"
            class="app-sidebar__settings"
            :class="{ 'is-active': route.path === '/settings' }"
            data-agent-id="sidebar.footer.settings"
            @click="router.push('/settings')"
          >
            设置
          </button>
        </NanaSidebarFooter>
      </NanaSidebarFrame>
      <main class="app-main">
        <NanaSessionProvider>
          <!-- NanaUI 渲染器对 RouterView 插槽内的组件替换补丁不生效,
               五页常驻挂载、v-show 切换行内可见性(上游修复后可换回 RouterView)。 -->
          <div v-show="route.path === '/'">
            <HomePage />
          </div>
          <div v-show="route.path === '/assistant'">
            <DanmakuAssistantPage />
          </div>
          <div v-show="route.path === '/stats'">
            <StatsPage />
          </div>
          <div v-show="route.path === '/history'">
            <HistoryPage />
          </div>
          <div v-show="route.path === '/settings'">
            <NanaSettingsPage />
          </div>
        </NanaSessionProvider>
      </main>
    </div>
  </NanaAppShell>
</template>

<style>
.app-layout {
  display: flex;
  height: 100%;
  min-height: 0;
}

.app-sidebar {
  width: 200px;
  flex: 0 0 auto;
}

.app-sidebar__settings {
  width: 100%;
  border: 0;
  background: transparent;
  color: inherit;
  font: inherit;
  font-size: 13px;
  padding: 8px 12px;
  text-align: left;
  cursor: pointer;
  border-radius: var(--radius-sm, 6px);
}

.app-sidebar__settings:hover {
  background: var(--bg-hover);
}

.app-main {
  flex: 1 1 auto;
  min-width: 0;
  height: 100%;
  overflow-y: auto;
  padding: 0 4px 0 0;
}

.app-main > * {
  min-height: 100%;
}
</style>
