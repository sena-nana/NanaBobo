<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import { useOperationalStore } from "../../../capabilities/operational";
import { useProductAdapter } from "../../../capabilities/product";
import { useErrorMapper } from "../../../capabilities/recovery";
import { CompletionFeedback, RecoveryError, SetupStep, SetupStepper } from "../../../ui/consumer";
import { OnboardingLayout } from "../../../ui/patterns";
import { createContinuation, type ContinuationSnapshot } from "../../../ui/state";

const storageKey = "tauri-template.onboarding";
const continuation = createContinuation({
  load: () => {
    try {
      const value = localStorage.getItem(storageKey);
      return value ? JSON.parse(value) as ContinuationSnapshot : null;
    } catch { return null; }
  },
  save: (snapshot) => localStorage.setItem(storageKey, JSON.stringify(snapshot)),
  clear: () => localStorage.removeItem(storageKey),
}, 5);
const router = useRouter();
const product = useProductAdapter();
const operational = useOperationalStore();
const mapError = useErrorMapper();
const currentStep = ref(continuation.resume());
const busy = ref(false);
const error = ref<unknown>();
const titles = ["选择使用方式", "检查设备", "选择资源", "测试主要能力", "完成设置"];
const recovery = computed(() => error.value ? mapError(error.value, continueStep) : null);

async function continueStep() {
  busy.value = true;
  error.value = undefined;
  try {
    if (currentStep.value === 2) {
      operational.update({ connection: "connecting" });
      await product.connectDevice();
      operational.update({ connection: "connected" });
    }
    if (currentStep.value === 4) await product.testPrimaryCapability();
    if (currentStep.value === 5) {
      continuation.complete(5);
      await router.push("/");
      return;
    }
    continuation.complete(currentStep.value);
    currentStep.value = continuation.snapshot.currentStep;
  } catch (cause) {
    error.value = cause;
    operational.update({ connection: "failed" });
  } finally {
    busy.value = false;
  }
}

function back() { currentStep.value = Math.max(1, currentStep.value - 1); }
async function skip() { continuation.skip(); await router.push("/"); }
onMounted(() => {
  if (continuation.snapshot.completedSteps.includes(5)) void router.replace("/");
});
</script>

<template>
  <OnboardingLayout
    :title="titles[currentStep - 1] ?? '首次设置'"
    :current-step="currentStep"
    :total-steps="titles.length"
    :can-continue="!busy"
    agent-id="onboarding.page"
    @continue="continueStep"
    @back="back"
    @skip="skip"
  >
    <SetupStepper label="首次设置进度">
      <SetupStep
        v-for="(title, index) in titles"
        :key="title"
        :title="title"
        :step="index + 1"
        :state="index + 1 < currentStep ? 'complete' : index + 1 === currentStep ? 'current' : 'pending'"
      />
    </SetupStepper>
    <p v-if="currentStep === 1">选择适合当前任务的默认工作方式，稍后可在设置中调整。</p>
    <p v-else-if="currentStep === 2">检查可用设备；失败时可以重试，也可以稍后继续。</p>
    <p v-else-if="currentStep === 3">Product Layer 在这里接入资源导入或选择流程。</p>
    <p v-else-if="currentStep === 4">运行一次主要能力测试，确认当前环境已准备好。</p>
    <p v-else>设置已完成，可以开始第一个任务。</p>
    <RecoveryError v-if="recovery" :model="recovery" presentation="inline" />
    <CompletionFeedback :trigger="currentStep" label="步骤已完成" agent-id="onboarding.completion" />
  </OnboardingLayout>
</template>
