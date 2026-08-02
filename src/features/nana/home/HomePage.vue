<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { RouterLink, useRouter } from "vue-router";
import { useOperationalStore } from "../../../capabilities/operational";
import { useProductAdapter, type RecentProject } from "../../../capabilities/product";
import { useErrorMapper } from "../../../capabilities/recovery";
import { useUndoManager } from "../../../capabilities/undo";
import { Button, Card, EmptyState, OperationalStatus } from "../../../ui";
import { DeviceStatusCard, OperationResult, RecoveryError, UndoableActionNotice } from "../../../ui/consumer";
import { HomeLayout, PrimaryActionArea } from "../../../ui/patterns";
import { createAsyncTaskController, initialReconnectState, reduceReconnect } from "../../../ui/state";

const product = useProductAdapter();
const router = useRouter();
const operational = useOperationalStore();
const mapError = useErrorMapper();
const undo = useUndoManager();
const recent = ref<readonly RecentProject[]>([]);
const removedName = ref("");
const loader = createAsyncTaskController<readonly RecentProject[]>();
const connection = ref(initialReconnectState);
const loadRecovery = computed(() => loader.error.value
  ? mapError(loader.error.value, loadRecent)
  : null);

async function loadRecent() {
  const projects = await loader.run(async () => await product.loadRecentProjects());
  if (projects) recent.value = projects;
}

async function connectDevice() {
  connection.value = reduceReconnect(connection.value, { type: connection.value.attempt ? "RETRY" : "CONNECT" });
  operational.update({ connection: connection.value.status });
  try {
    await product.connectDevice();
    connection.value = reduceReconnect(connection.value, { type: "CONNECTED" });
  } catch (error) {
    connection.value = reduceReconnect(connection.value, { type: "FAILED", error });
  }
  operational.update({ connection: connection.value.status });
}

async function removeProject(project: RecentProject) {
  const index = recent.value.findIndex((item) => item.id === project.id);
  await undo.execute({
    id: `remove-${project.id}`,
    execute: () => { recent.value = recent.value.filter((item) => item.id !== project.id); },
    undo: () => {
      const restored = [...recent.value];
      restored.splice(index, 0, project);
      recent.value = restored;
    },
  });
  removedName.value = project.name;
}

async function restoreRemoved() {
  if (await undo.undo()) removedName.value = "";
}

onMounted(() => { void loadRecent(); });
</script>

<template>
  <HomeLayout title="开始工作" description="选择最近项目，或创建一个新的工作区。" agent-id="home.page">
    <template #status><OperationalStatus :state="operational.state" /></template>
    <template #primary-action>
      <PrimaryActionArea><Button variant="primary" @click="router.push('/editor')">快速开始</Button></PrimaryActionArea>
    </template>
    <template #recent>
      <Card agent-id="home.recent"><h2>最近项目</h2>
        <p v-if="loader.status.value === 'running'">正在加载…</p>
        <RecoveryError v-else-if="loadRecovery" :model="loadRecovery" presentation="inline" />
        <EmptyState v-else-if="recent.length === 0" title="还没有项目" description="创建项目后会显示在这里。">
          <template #actions><Button @click="loadRecent">重新加载</Button></template>
        </EmptyState>
        <ul v-else class="recent-list">
          <li v-for="project in recent" :key="project.id">
            <RouterLink :to="`/editor?project=${project.id}`">{{ project.name }}</RouterLink>
            <span>{{ project.updatedAt }}</span>
            <Button variant="text" @click="removeProject(project)">移除</Button>
          </li>
        </ul>
      </Card>
    </template>
    <template #device>
      <DeviceStatusCard title="设备" :state="connection.status" description="连接后可开始主要任务。" agent-id="home.device">
        <Button :loading="connection.status === 'connecting' || connection.status === 'reconnecting'" @click="connectDevice">
          {{ connection.status === "failed" ? "重新连接" : "连接设备" }}
        </Button>
      </DeviceStatusCard>
    </template>
    <template #next-step><Card><h2>推荐下一步</h2><p>打开编辑页，确认内容和输出状态。</p></Card></template>
    <template #history><OperationResult title="模板已准备好" description="页面数据和操作可由 Product Adapter 替换。" /></template>
  </HomeLayout>
  <UndoableActionNotice v-if="removedName" :message="`已移除 ${removedName}`" @undo="restoreRemoved" @expired="removedName = ''" />
</template>

<style scoped>
.recent-list { display: grid; gap: 6px; margin: 10px 0 0; padding: 0; list-style: none; }
.recent-list li { display: flex; align-items: center; gap: 8px; min-width: 0; }
.recent-list a { min-width: 0; flex: 1; overflow: hidden; color: var(--text); text-overflow: ellipsis; white-space: nowrap; }
.recent-list span { color: var(--text-muted); font-size: 12px; }
</style>
