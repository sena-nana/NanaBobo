import { createApp } from "vue";
import {
  createRouter,
  createWebHistory,
  type RouterHistory,
} from "vue-router";
import AppRoot from "./AppRoot.vue";
import { commands } from "./commands";
import { installCommandRegistry } from "./ui/commands";
import { activeUIPreset, type TemplateUIPresetAdapter } from "./ui/preset";

export function createTemplateApp(
  history?: RouterHistory,
  preset: TemplateUIPresetAdapter = activeUIPreset,
) {
  const app = createApp(AppRoot, {
    provider: preset.provider,
    policy: preset.policy,
    hosts: preset.hosts,
  });
  const router = createTemplateRouter(history, preset);

  preset.install?.(app);
  for (const capability of preset.appCapabilities ?? []) capability.install(app);
  installCommandRegistry(app, commands);
  app.use(router);
  if (
    import.meta.env.DEV
    && (import.meta.env.VITE_LILIA_AGENT_DEBUG === "1" || import.meta.env.MODE === "agent-debug")
  ) {
    void preset.installDiagnostics?.();
  }

  return { app, router };
}

export function createTemplateRouter(
  history?: RouterHistory,
  preset: TemplateUIPresetAdapter = activeUIPreset,
) {
  return createRouter({
    history: history ?? createWebHistory(),
    routes: [
      {
        path: "/",
        component: preset.shell,
        props: preset.shellProps,
        meta: { sidebar: "main", returnable: true },
        children: [...preset.routes],
      },
      { path: "/:pathMatch(.*)*", redirect: "/" },
    ],
  });
}
