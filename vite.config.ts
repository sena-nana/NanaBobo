/// <reference types="vitest" />
import { defineLiliaViteConfig } from "@lilia/config";
import { configDefaults } from "vitest/config";
import appConfig from "./app.config.json";
import { uiBundleGuard } from "./scripts/ui-bundle-guard";

const unitTestExclude = [
  ...configDefaults.exclude,
  "tests/e2e/**",
  ...(appConfig.ui?.preset === "nana" ? [] : ["tests/capabilities.test.ts"]),
];

export default defineLiliaViteConfig({
  plugins: [uiBundleGuard(appConfig.ui?.preset ?? "lilia")],
  test: { exclude: unitTestExclude },
});
