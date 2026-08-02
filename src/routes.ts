import type { RouteRecordRaw } from "vue-router";
import { activeUIPreset } from "./ui/preset";

export const routes: readonly RouteRecordRaw[] = activeUIPreset.routes;
