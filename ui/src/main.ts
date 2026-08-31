import { createApp } from "@nanaui/nanavue-runtime";
import SpikeApp from "./SpikeApp.vue";
import { nanaHost } from "./nanaHost";
import "./spike.css";

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

(globalThis as Record<string, unknown>).__nanabobo = {
  mount() {
    createApp(SpikeApp).mount();
    return { mounted: true };
  },
};
