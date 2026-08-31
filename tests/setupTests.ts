import { JSDOM } from "jsdom";
import { afterEach } from "vitest";

// Node 26 的实验性全局 localStorage(未提供 --localstorage-file 时不可用)会以
// getter 形式占据 globalThis,导致 vitest 的 jsdom 环境跳过同名键的注入
// (全局已存在时不覆盖)。这里显式用 jsdom 的 Storage 接管 globalThis.localStorage。
const storageDom = new JSDOM("", { url: "http://localhost/" });
Object.defineProperty(globalThis, "localStorage", {
  value: storageDom.window.localStorage,
  configurable: true,
  writable: true,
});

// 服务模块在导入期构建默认 Nana 传输;jsdom 下没有宿主桥,这里补一个空实现,
// 让默认 bilibiliApi 可被安全导入(用例内部仍注入各自的假实现)。
(globalThis as Record<string, unknown>).__nanaHost ??= {
  call: () => undefined,
  invoke: async () => undefined,
  on: () => () => {},
};

afterEach(() => {
  if (typeof localStorage !== "undefined") localStorage.clear();
  if (typeof document !== "undefined") {
    document.documentElement.removeAttribute("data-corners");
    document.documentElement.removeAttribute("data-theme");
    document.documentElement.style.removeProperty("--app-corner-radius");
  }
});
