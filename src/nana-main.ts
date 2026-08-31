import { createApp } from "@nanaui/nanavue-runtime";
import { defineComponent, h } from "vue";
import {
  createMemoryHistory,
  createRouter,
  RouterView,
  type RouteRecordRaw,
} from "vue-router";
import {
  installCornerStyle,
  installLiliaContextMenu,
  installNativeAppearance,
  provideLiliaSettings,
  setLiliaUiConfig,
} from "@nanaui/nanavue-components/appearance";
import NanaShell from "./ui/nana/NanaShell.vue";
import { settingsModel } from "./ui/nana/settings";
import { nanaHost } from "./ui/nanaHost";
import HomePage from "./features/home/HomePage.vue";
import DanmakuAssistantPage from "./features/danmaku/DanmakuAssistantPage.vue";
import StatsPage from "./features/stats/StatsPage.vue";
import HistoryPage from "./features/history/HistoryPage.vue";
import NanaSettingsPage from "@nanaui/nanavue-components/NanaSettingsPage";
import "./ui/nana-styles.css";
import appConfig from "../app.config.json";

const bridge = nanaHost();

// localStorage 默认仅驻内存;写穿到宿主文件,重启后由宿主回灌。
const nativeStorage = globalThis.localStorage;
const seed = (bridge.call("storage_load") ?? {}) as Record<string, string>;
for (const [key, value] of Object.entries(seed)) {
  if (nativeStorage.getItem(key) !== value) {
    nativeStorage.setItem(key, value);
  }
}
globalThis.localStorage = {
  getItem: (key: string) => nativeStorage.getItem(String(key)),
  setItem: (key: string, value: string) => {
    nativeStorage.setItem(String(key), String(value));
    bridge.call("storage_save", [String(key), String(value)]);
  },
  removeItem: (key: string) => {
    nativeStorage.removeItem(String(key));
    bridge.call("storage_remove", [String(key)]);
  },
  clear: () => {
    nativeStorage.clear();
    bridge.call("storage_clear");
  },
  key: (index: number) => nativeStorage.key(index),
  get length() {
    return nativeStorage.length;
  },
} as Storage;

function createNanaRouter() {
  const routes: RouteRecordRaw[] = [
    {
      path: "/",
      component: NanaShell,
      meta: { sidebar: "main" },
      children: [
        { path: "", component: HomePage },
        { path: "assistant", component: DanmakuAssistantPage },
        { path: "stats", component: StatsPage },
        { path: "history", component: HistoryPage },
        { path: "settings", component: NanaSettingsPage },
      ],
    },
    { path: "/:pathMatch(.*)*", redirect: "/" },
  ];
  return createRouter({ history: createMemoryHistory(), routes });
}

const RootView = defineComponent({
  name: "RootView",
  setup: () => () => h(RouterView),
});

(globalThis as Record<string, unknown>).__nanabobo = {
  mount() {
    setLiliaUiConfig({
      appName: appConfig.appName,
      productTitle: appConfig.productTitle,
      version: appConfig.version,
      identifier: appConfig.identifier,
      storageKeyPrefix: appConfig.storageKeyPrefix,
    });
    installNativeAppearance();
    installCornerStyle();

    const app = createApp(RootView);
    app.config.errorHandler = (error: unknown, _instance, info) => {
      bridge.call("spike_echo", [`error=${String(error)} info=${info}`]);
    };
    const router = createNanaRouter();
    provideLiliaSettings(app, settingsModel);
    installLiliaContextMenu(app);
    app.use(router);
    void router.isReady().then(() => {
      const current = router.currentRoute.value;
      bridge.call("spike_echo", [
        `route=${current.fullPath} matched=${current.matched.length} name=${String(current.name)}`,
      ]);
      app.mount();
    });
    return { mounted: true };
  },
};
