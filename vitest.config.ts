import { fileURLToPath } from "node:url";
import vue from "@vitejs/plugin-vue";
import { defineConfig } from "vitest/config";

// 测试直接消费 .nanaui-pin 快照的包源码,与 ui/vite.config.ts 的别名保持一致。
// pin 源码统一从 @vue/runtime-core 导入;测试里统一改走 "vue",
// 保证 pin 组件与应用代码共用同一个运行时实例。
const nanaPackages = fileURLToPath(new URL("../.nanaui-pin/packages", import.meta.url));

export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: [
      { find: /^@nanaui\/nanavue-components$/, replacement: `${nanaPackages}/nanavue-components/src/index.js` },
      { find: /^@nanaui\/nanavue-components\/(.+)$/, replacement: `${nanaPackages}/nanavue-components/src/$1.js` },
      { find: "@nanaui/nanavue-runtime", replacement: `${nanaPackages}/nanavue-runtime/src/createNanaRenderer.js` },
      { find: "@vue/runtime-core", replacement: "vue" },
    ],
  },
  test: {
    environment: "jsdom",
    setupFiles: ["tests/setupTests.ts"],
    include: ["tests/**/*.test.ts"],
  },
});
