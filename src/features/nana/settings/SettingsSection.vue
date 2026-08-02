<script setup lang="ts">
import { computed, ref } from "vue";
import { Button, Switch, useNanaUI } from "../../../ui";
import { AdvancedSettingsDisclosure, ProgressiveSection } from "../../../ui/consumer";

defineProps<{ title: string; description: string; showAdvanced?: boolean }>();
const enabled = ref(true);
const advancedEnabled = ref(false);
const ui = useNanaUI();
const compact = computed({
  get: () => ui.policy.value.density === "compact",
  set: (value) => ui.setPolicy({ density: value ? "compact" : "comfortable" }),
});

function restoreDefaults() {
  enabled.value = true;
  advancedEnabled.value = false;
}
</script>

<template>
  <ProgressiveSection title="常用设置" level="essential" :agent-id="`settings.${title}.essential`">
    <p>{{ description }}</p>
    <Switch v-if="title === '外观'" v-model="compact" label="紧凑密度" agent-id="settings.density" />
    <Switch v-model="enabled" :label="`启用${title}`" />
  </ProgressiveSection>
  <ProgressiveSection v-if="showAdvanced" level="optional" title="可选项">
    <AdvancedSettingsDisclosure>
      <Switch v-model="advancedEnabled" label="使用高级选项" />
    </AdvancedSettingsDisclosure>
  </ProgressiveSection>
  <Button variant="text" @click="restoreDefaults">恢复默认</Button>
</template>
