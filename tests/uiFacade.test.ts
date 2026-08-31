import { readFileSync, readdirSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

function sourceFiles(directory: string): string[] {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const path = resolve(directory, entry.name);
    return entry.isDirectory() ? sourceFiles(path) : /\.(?:ts|vue)$/.test(entry.name) ? [path] : [];
  });
}

describe("UI facade boundary", () => {
  it("keeps feature code independent from concrete UI layers", () => {
    const concreteLayer = `${"@"}nanaui`;
    const directImport = new RegExp(`from\\s+["']${concreteLayer}(?:/|["'])`);
    const violations = sourceFiles(resolve("src/features")).filter((path) =>
      directImport.test(readFileSync(path, "utf8")),
    );
    expect(violations).toEqual([]);
  });
});
