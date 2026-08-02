import type { App, Component } from "vue";
import type { RouteRecordRaw } from "vue-router";
import type { AppUIPresetAdapter } from "./contract";

export interface TemplateAppCapability {
  id: string;
  install: (app: App) => void;
}

export interface TemplateUIPresetAdapter extends AppUIPresetAdapter<Component> {
  routes: readonly RouteRecordRaw[];
  shellProps?: Readonly<Record<string, unknown>>;
  hosts?: Component;
  appCapabilities?: readonly TemplateAppCapability[];
  install?: (app: App) => void;
  installDiagnostics?: () => Promise<boolean>;
}
