// A made mod's card title: a tab reads as its tab does, every other kind as the Mods page titles it.

import { describe, expect, test } from "vitest";

import { madeTitle } from "./mods-made";

describe("madeTitle", () => {
  test("a tab reads as its tab does", () => {
    expect(madeTitle("review-queue", "tab")).toBe("Review queue");
  });

  test("a theme reads as the Mods page titles it", () => {
    expect(madeTitle("dusk", "theme")).toBe("Dusk");
  });

  test("a built-in theme drops its prefix", () => {
    expect(madeTitle("built-in:calm", "theme")).toBe("Calm");
  });
});
