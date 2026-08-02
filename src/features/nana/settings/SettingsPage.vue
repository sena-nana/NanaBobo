<script setup lang="ts">
import { computed } from "vue";
import { useRoute, useRouter } from "vue-router";
import { Tabs } from "../../../ui";
import { SettingsLayout } from "../../../ui/patterns";
import { normalizeSettingsTab, useSettings } from "../../../ui/settings";

const route = useRoute();
const router = useRouter();
const installedSettings = useSettings();
if (!installedSettings) throw new Error("Settings capability is not installed.");
const settings = installedSettings;
const activeKey = computed(() => normalizeSettingsTab(settings, route.query.tab));
const activeSection = computed(() => settings.sections[activeKey.value]);
const activeProps = computed(() => settings.sectionProps[activeKey.value] ?? {});
const options = settings.tabs.map((tab) => ({ value: tab.key, label: tab.label }));

function selectTab(value: string) {
  void router.replace({ path: settings.path, query: { tab: value } });
}
</script>

<template>
  <SettingsLayout title="设置" :description="settings.description" agent-id="settings.page">
    <template #navigation>
      <Tabs :model-value="activeKey" :options="options" orientation="vertical" agent-id="settings.navigation" @update:model-value="selectTab" />
    </template>
    <Suspense>
      <component :is="activeSection" v-bind="activeProps" />
      <template #fallback><p role="status">正在加载设置…</p></template>
    </Suspense>
  </SettingsLayout>
</template>
