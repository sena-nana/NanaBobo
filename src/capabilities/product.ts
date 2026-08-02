import { inject, type InjectionKey } from "vue";
import type { TemplateAppCapability } from "../ui/types";

export interface RecentProject {
  id: string;
  name: string;
  updatedAt: string;
}

export interface ProjectDraft {
  id: string;
  name: string;
  notes: string;
}

export interface ProductAdapter {
  connectDevice: () => Promise<void>;
  loadRecentProjects: () => Promise<readonly RecentProject[]>;
  saveProject: (draft: ProjectDraft) => Promise<void>;
  testPrimaryCapability: () => Promise<void>;
}

const productAdapterKey: InjectionKey<ProductAdapter> = Symbol("templateProductAdapter");

export function productCapability(adapter: ProductAdapter): TemplateAppCapability {
  return { id: "product-adapter", install: (app) => app.provide(productAdapterKey, adapter) };
}

export function useProductAdapter(): ProductAdapter {
  const adapter = inject(productAdapterKey);
  if (!adapter) throw new Error("Product adapter capability is not installed.");
  return adapter;
}

export function createMockProductAdapter(options: { failConnectOnce?: boolean } = {}): ProductAdapter {
  let failConnectOnce = options.failConnectOnce ?? false;
  const projects: RecentProject[] = [
    { id: "welcome", name: "欢迎项目", updatedAt: "刚刚" },
    { id: "sample", name: "示例工作区", updatedAt: "昨天" },
  ];
  return {
    async connectDevice() {
      if (failConnectOnce) {
        failConnectOnce = false;
        throw new Error("Mock device is temporarily unavailable.");
      }
      await Promise.resolve();
    },
    async loadRecentProjects() { return projects.map((project) => ({ ...project })); },
    async saveProject(draft) {
      if (!draft.name.trim()) throw new Error("Project name is required.");
      const existing = projects.find((project) => project.id === draft.id);
      if (existing) existing.name = draft.name;
      await Promise.resolve();
    },
    async testPrimaryCapability() { await Promise.resolve(); },
  };
}
