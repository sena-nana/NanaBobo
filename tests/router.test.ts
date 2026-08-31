import { describe, expect, it } from "vitest";
import { createNanaRouter } from "../src/ui/nana/createRouter";

describe("NanaBobo routes", () => {
  it("registers the real home workflow and every tool page", async () => {
    const router = createNanaRouter();
    await router.push("/");
    await router.isReady();

    expect(router.currentRoute.value.path).toBe("/");
    expect(router.currentRoute.value.matched[0]?.components?.default).toBeDefined();

    for (const path of ["/assistant", "/stats", "/history", "/settings"] as const) {
      await router.push(path);
      expect(router.currentRoute.value.path).toBe(path);
    }
  });

  it("redirects unknown paths to the real home route", async () => {
    const router = createNanaRouter();
    await router.push("/missing");
    expect(router.currentRoute.value.path).toBe("/");
  });
});
