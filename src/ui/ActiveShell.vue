<script setup lang="ts">
import { computed } from "vue";
import { RouterView, useRoute } from "vue-router";
import { resolveBackdropSurfaces, useNativeAppearance } from "@lilia/ui/composables";
import { normalizeSettingsTab, useSettings } from "@lilia/ui-foundation/settings";
import { LiliaPrimaryContent, LiliaSectionNavigation, LiliaWorkspace } from "@lilia/ui/layouts";
import { LiliaSettingsSidebar } from "@lilia/ui/settings/sidebar";
import { LiliaAppShell } from "@lilia/ui/shell/app";
import { resolveLiliaIcon, type SidebarNavItem } from "@lilia/ui/shell/config";
import { LiliaSidebarFrame, LiliaSidebarNavRow } from "@lilia/ui/shell/sidebar";

const route = useRoute();
const settings = useSettings();
const appearance = useNativeAppearance();
const surfaces = computed(() => resolveBackdropSurfaces(
  appearance.backdropMode.value,
  appearance.backdropTarget.value,
));
const settingsMode = computed(() => settings !== null && route.path === settings.path);
const activeSettingsTab = computed(() => settings ? normalizeSettingsTab(settings, route.query.tab) : "");
const sidebarNav: SidebarNavItem[] = [
  { key: "overview", to: "/", label: "首页", icon: resolveLiliaIcon("home") },
];
</script>

<template>
  <LiliaAppShell>
    <LiliaWorkspace
      aria-label="应用工作区"
      :surface-mode="surfaces.workspace"
      backdrop-effect="none"
    >
      <LiliaSectionNavigation
        id="nanabobo-navigation"
        :surface-mode="surfaces.sidebar"
      >
        <LiliaSettingsSidebar
          v-if="settingsMode && settings"
          :tabs="settings.tabs"
          :active-key="activeSettingsTab"
          return-to="/"
          :surface-mode="surfaces.sidebar"
        />
        <LiliaSidebarFrame
          v-else
          aria-label="主导航"
          :surface-mode="surfaces.sidebar"
        >
          <nav class="nanabobo-sidebar-nav" aria-label="主导航">
            <LiliaSidebarNavRow
              v-for="item in sidebarNav"
              :key="item.key"
              :item="item"
              :agent-id="`sidebar.nav.${item.key}`"
              :emphasis="item.emphasis"
            />
          </nav>
        </LiliaSidebarFrame>
      </LiliaSectionNavigation>
      <LiliaPrimaryContent
        id="nanabobo-primary"
        :surface-mode="surfaces.main"
      >
        <RouterView />
      </LiliaPrimaryContent>
    </LiliaWorkspace>
  </LiliaAppShell>
</template>

<style scoped>
.nanabobo-sidebar-nav { display: flex; flex-direction: column; gap: 1px; min-height: 0; }
</style>
