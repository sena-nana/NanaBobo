import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import { fileURLToPath, pathToFileURL } from "node:url";
import { resolve } from "node:path";

const runtimeCore = fileURLToPath(
  new URL("./node_modules/@vue/runtime-core/dist/runtime-core.esm-bundler.js", pathToFileURL(resolve(process.cwd(), "vite.config.ts")).href),
);
const nanaPackages = resolve(process.cwd(), "../../.nanaui-pin/packages");

export default defineConfig({
  plugins: [vue()],
  resolve: {
    // SFC 编译助手来自 runtime-core;createApp 由 Nana 渲染器提供,
    // runtime-dom 被刻意排除出 bundle。入口在 ui/ 之外,@nanaui/*
    // 用别名直接指向 pin 快照的包源码。
    alias: [
      { find: /^@nanaui\/nanavue-components$/, replacement: `${nanaPackages}/nanavue-components/src/index.js` },
      { find: /^@nanaui\/nanavue-components\/(.+)$/, replacement: `${nanaPackages}/nanavue-components/src/$1.js` },
      { find: "@nanaui/nanavue-runtime", replacement: `${nanaPackages}/nanavue-runtime/src/createNanaRenderer.js` },
      { find: "vue", replacement: fileURLToPath(new URL("./vue-shim.ts", import.meta.url)) },
      { find: "@vue/runtime-core", replacement: runtimeCore },
    ],
  },
  build: {
    target: "es2020",
    cssCodeSplit: false,
    emptyOutDir: true,
    lib: {
      entry: "../src/nana-main.ts",
      name: "NanaBoboUi",
      formats: ["iife"],
      fileName: () => "nanabobo.iife.js",
    },
  },
});
