<script setup lang="ts">
import AccountPanel from "../account/AccountPanel.vue";
import RoomInfoPanel from "../live/RoomInfoPanel.vue";
</script>

<template>
  <section class="page" data-agent-id="home.page">
    <div class="page-header" data-agent-id="home.header">
      <div>
        <h1>Nana播播工具箱</h1>
        <p>管理B站账号，快速查看直播间信息。</p>
      </div>
    </div>
    <div class="feature-grid">
      <AccountPanel />
      <RoomInfoPanel />
    </div>
  </section>
</template>

<style scoped>
.page { display: grid; gap: 18px; }
.feature-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 16px; }
@media (max-width: 820px) { .feature-grid { grid-template-columns: 1fr; } }
</style>
