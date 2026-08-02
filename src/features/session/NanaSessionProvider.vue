<script setup lang="ts">
import { provideNanaSession } from "./useNanaSession";

provideNanaSession();
</script>

<template>
  <slot />
</template>
