<script setup lang="ts">
import type { Component } from "vue";
import { RouterView } from "vue-router";
import type { UIPolicy } from "./ui/contract";
import NanaSessionProvider from "./features/session/NanaSessionProvider.vue";

defineProps<{ provider?: Component; policy: UIPolicy; hosts?: Component }>();
</script>

<template>
  <component :is="provider" v-if="provider" :policy="policy">
    <NanaSessionProvider>
      <RouterView />
      <component :is="hosts" v-if="hosts" />
    </NanaSessionProvider>
  </component>
  <template v-else>
    <NanaSessionProvider>
      <RouterView />
      <component :is="hosts" v-if="hosts" />
    </NanaSessionProvider>
  </template>
</template>
