<script setup lang="ts">
import { computed, onBeforeUnmount, reactive, ref, watch } from "vue";
import { useRoute } from "vue-router";
import { useTemplateAutoSave } from "../../../capabilities/autoSave";
import { useOperationalStore } from "../../../capabilities/operational";
import { useProductAdapter, type ProjectDraft } from "../../../capabilities/product";
import { useErrorMapper } from "../../../capabilities/recovery";
import { Button, Card, Input, ListItem, OperationalStatus, Progress, Textarea } from "../../../ui";
import { RecoveryError } from "../../../ui/consumer";
import { ContextPanel, EditorLayout, PersistentStatusBar } from "../../../ui/patterns";

const route = useRoute();
const product = useProductAdapter();
const operational = useOperationalStore();
const mapError = useErrorMapper();
const selectedObject = ref<string | null>("scene");
const lastSaveError = ref<unknown>();
const draft = reactive<ProjectDraft>({
  id: typeof route.query.project === "string" ? route.query.project : "welcome",
  name: "欢迎项目",
  notes: "",
});
const autoSave = useTemplateAutoSave({
  serialize: () => ({ ...draft }),
  save: async (snapshot) => {
    try {
      await product.saveProject(snapshot);
      lastSaveError.value = undefined;
    } catch (error) {
      lastSaveError.value = error;
      throw error;
    }
  },
});
const recovery = computed(() => lastSaveError.value
  ? mapError(lastSaveError.value, async () => { await autoSave.retry(); })
  : null);

function changeName(value: string) { draft.name = value; autoSave.markDirty(); }
function changeNotes(value: string) { draft.notes = value; autoSave.markDirty(); }
async function saveNow() { await autoSave.flush(); }
function toggleRuntime() {
  operational.update({ runtime: operational.state.runtime === "running" ? "paused" : "running" });
}

watch(autoSave.state, (state) => { if (state !== "failed") lastSaveError.value = undefined; });
onBeforeUnmount(() => operational.update({ runtime: "idle" }));
</script>

<template>
  <EditorLayout :context-visible="selectedObject !== null" status-visible agent-id="editor.page">
    <template #top>
      <div class="editor-top">
        <strong>{{ draft.name || "未命名项目" }}</strong>
        <OperationalStatus :state="operational.state" />
        <Button @click="saveNow">立即保存</Button>
        <Button variant="primary" @click="toggleRuntime">{{ operational.state.runtime === "running" ? "暂停" : "开始运行" }}</Button>
      </div>
    </template>
    <template #objects>
      <nav aria-label="场景对象">
        <ListItem :active="selectedObject === 'scene'" @select="selectedObject = 'scene'">主场景</ListItem>
        <ListItem :active="selectedObject === 'output'" @select="selectedObject = 'output'">输出</ListItem>
        <Button v-if="selectedObject" variant="text" @click="selectedObject = null">取消选择</Button>
      </nav>
    </template>
    <template #preview>
      <Card><h2>实时预览</h2><p>Product Layer 在此接入预览或主要内容。</p></Card>
      <RecoveryError v-if="recovery" :model="recovery" presentation="inline" agent-id="editor.recovery" />
    </template>
    <template #context>
      <ContextPanel :title="selectedObject === 'scene' ? '场景属性' : '输出属性'" agent-id="editor.context">
        <label>项目名称<Input :model-value="draft.name" aria-label="项目名称" @update:model-value="changeName(String($event))" /></label>
        <label>备注<Textarea :model-value="draft.notes" aria-label="备注" @update:model-value="changeNotes(String($event))" /></label>
      </ContextPanel>
    </template>
    <template #status>
      <PersistentStatusBar label="编辑状态">
        <OperationalStatus :state="operational.state" />
        <Progress v-if="operational.state.runtime === 'running'" label="运行中" />
      </PersistentStatusBar>
    </template>
  </EditorLayout>
</template>

<style scoped>
.editor-top { display: flex; align-items: center; gap: 10px; min-width: 0; }
.editor-top strong { min-width: 0; flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
nav { display: grid; gap: 4px; }
label { display: grid; gap: 5px; margin-bottom: 12px; color: var(--text-muted); font-size: 12px; }
</style>
