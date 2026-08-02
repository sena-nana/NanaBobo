import { liliaPresetDefinition } from "@lilia/ui/preset/definition";
import { provideSettings, createSettingsModel } from "@lilia/ui-foundation/settings";
import { resolveLiliaIcon, setLiliaUiConfig } from "@lilia/ui/shell/config";
import { installCornerStyle, installGlobalScrollbarVisibility, installLiliaContextMenu, installNativeAppearance } from "@lilia/ui/runtime";
import { installTauriNativeAppearanceAdapter } from "@lilia/ui/runtime/tauri";
import ContextMenuHost from "@lilia/ui/components/ContextMenuHost";
import OverlayHost from "@lilia/ui/components/OverlayHost";
import appConfigJson from "../../app.config.json";
import { defineComponent, h, type Component } from "vue";
import type { AppUIPresetAdapter } from "./contract";
import type { NanaBoboUIPresetAdapter } from "./types";
import ActiveShell from "./ActiveShell.vue";

const upstream = liliaPresetDefinition as AppUIPresetAdapter<Component>;
const Hosts = defineComponent({ setup: () => () => [h(ContextMenuHost), h(OverlayHost)] });
const settings = createSettingsModel({
  path: "/settings", defaultTab: "appearance", description: "偏好设置会保存到本地。",
  tabs: [
    { key: "appearance", label: "外观", icon: resolveLiliaIcon("palette") },
    { key: "about", label: "关于", icon: resolveLiliaIcon("info") },
  ],
  sections: {
    appearance: () => import("@lilia/ui/settings").then((module) => ({ default: module.LiliaAppearanceSection })),
    about: () => import("@lilia/ui/settings").then((module) => ({ default: module.LiliaAboutSection })),
  },
});

export const nanaBoboPreset: NanaBoboUIPresetAdapter = {
  ...upstream,
  shell: ActiveShell,
  routes: [
    { path: "", component: () => import("../features/home/HomePage.vue") },
    { path: "settings", component: () => import("@lilia/ui/settings").then((module) => module.LiliaSettingsPage) },
  ],
  hosts: Hosts,
  install(app) {
    setLiliaUiConfig({
      appName: appConfigJson.appName, productTitle: appConfigJson.productTitle,
      version: appConfigJson.version, storageKeyPrefix: appConfigJson.storageKeyPrefix,
      appearance: { backdropTarget: "sidebar" },
      sidebar: {
        footerLinks: [{ key: "settings", to: "/settings", label: "设置", icon: "settings" }],
      },
    });
    provideSettings(app, settings);
    installLiliaContextMenu(app);
    installGlobalScrollbarVisibility();
    installCornerStyle();
    installTauriNativeAppearanceAdapter();
    installNativeAppearance();
  },
  installDiagnostics: async () => {
    const diagnostics = await import("@lilia/ui/diagnostics");
    if (!diagnostics.isLiliaAgentDebugEnabled()) return false;
    diagnostics.installAgentDebugHarness();
    return true;
  },
};
