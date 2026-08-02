import { defineToolsProfile } from "@lilia/tools";
import { readFileSync } from "node:fs";

const preset = JSON.parse(readFileSync(new URL("./app.config.json", import.meta.url), "utf8")).ui?.preset ?? "lilia";
const isNana = preset === "nana";

export default defineToolsProfile({
  requireSingleAppRoot: true,
  expectedDependencies: [
    "@lilia/build",
    "@lilia/config",
    "@lilia/theme",
    "@lilia/tools",
    isNana ? "@lilia/nana-ui" : "@lilia/ui",
    "@lilia/ui-contract",
    "@lilia/ui-foundation",
    "vue",
    "vue-router",
  ],
  nativeBackdropPermissions: [
    "lilia:default",
    "lilia:allow-set-window-backdrop",
  ],
  importantFiles: [
    ["app.config.json", "application metadata source"],
    ["src/main.ts", "Vue mount entry"],
    ["src/AppRoot.vue", "application-owned root and global hosts"],
    ["src/app.ts", "Vue, Router, Shell, commands, and provider assembly"],
    ["src/routes.ts", "application routes"],
    ["src/commands.ts", "application command map"],
    ["src/overlays.ts", "application overlay composition"],
    ["src/ui/index.ts", "active UI facade"],
    ["src/ui/preset.ts", "active preset adapter"],
    ["src/features/home/HomePage.vue", "default application page"],
    ["src/features/account/AccountPanel.vue", "Bilibili account workflow"],
    ["src/features/live/RoomInfoPanel.vue", "live room lookup workflow"],
    ["tests/app.test.ts", "explicit application assembly test"],
    ["tests/tooling.test.ts", "NanaBobo tooling contract tests"],
    ["docs/guide/development.md", "development workflow"],
  ],
  agentTargetFiles: {
    "src/features/home/HomePage.vue": [
      ["home.page"],
      ["home.header"],
    ],
    "src/features/account/AccountPanel.vue": [
      ["account.panel"],
      ["account.login"],
      ["account.logout"],
      ["account.qr"],
    ],
    "src/features/live/RoomInfoPanel.vue": [
      ["room.panel"],
      ["room.input"],
      ["room.query"],
      ["room.info"],
    ],
  },
  boundaries: {
    includes: [
      "application bootstrap and routing",
      "application configuration and commands",
      "application-owned features and Tauri boundaries",
    ],
    excludes: [
      "shared UI and shell implementations",
      "shared build and tooling engines",
      "shared Tauri window runtime",
    ],
  },
  entrypoints: [
    { id: "dev", command: "yarn dev", purpose: "start the frontend development server" },
    { id: "agent-debug", command: "yarn agent:debug --json", purpose: "inspect NanaBobo readiness" },
    { id: "test", command: "yarn test", purpose: "run application behavior tests" },
    { id: "verify", command: "yarn verify", purpose: "run the complete application verification" },
  ],
});
