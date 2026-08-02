import { readFileSync, readdirSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import { activeUIPreset } from "../src/ui/preset";
import appConfig from "../app.config.json";

function sourceFiles(directory: string): string[] {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const path = resolve(directory, entry.name);
    return entry.isDirectory() ? sourceFiles(path) : /\.(?:ts|vue)$/.test(entry.name) ? [path] : [];
  });
}

describe("UI facade boundary", () => {
  it("keeps feature code independent from concrete UI layers", () => {
    const concreteLayer = `${"@"}lilia/(?:ui|nana-ui)`;
    const directImport = new RegExp(`from\\s+["']${concreteLayer}(?:/|["'])`);
    const violations = sourceFiles(resolve("src/features")).filter((path) =>
      directImport.test(readFileSync(path, "utf8")),
    );
    expect(violations).toEqual([]);
  });

  it("registers every active page as a lazy route", () => {
    expect(activeUIPreset.id).toBe(appConfig.ui?.preset);
    expect(activeUIPreset.routes.length).toBeGreaterThanOrEqual(activeUIPreset.id === "nana" ? 4 : 2);
    expect(activeUIPreset.routes.every((route) => typeof route.component === "function")).toBe(true);
  });
});
