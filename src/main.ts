import { createApp } from "@nanaui/nanavue-runtime";
import {
  installCornerStyle,
  installLiliaContextMenu,
  installNativeAppearance,
  provideLiliaSettings,
  setLiliaUiConfig,
} from "@nanaui/nanavue-components/appearance";
import NanaShell from "./ui/nana/NanaShell.vue";
import { createNanaRouter } from "./ui/nana/createRouter";
import { settingsModel } from "./ui/nana/settings";
import { nanaHost } from "./ui/nanaHost";
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

let activeRouter: ReturnType<typeof createNanaRouter> | undefined;

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

    const app = createApp(NanaShell);
    app.config.errorHandler = (error: unknown, _instance, info) => {
      bridge.call("spike_echo", [`error=${String(error)} info=${info}`]);
    };
    const router = createNanaRouter();
    activeRouter = router;
    provideLiliaSettings(app, settingsModel);
    installLiliaContextMenu(app);
    app.use(router);
    void router.isReady().then(() => {
      app.mount();
    });
  },
  /** 供宿主无头测试逐页导航。 */
  navigate(path: string) {
    if (!activeRouter) return;
    void activeRouter
      .push(path)
      .then(() => {
        bridge.call("spike_echo", [`nav=${activeRouter?.currentRoute.value.fullPath}`]);
      })
      .catch((error: unknown) => {
        bridge.call("spike_echo", [`nav-error=${String(error)}`]);
      });
  },
};
