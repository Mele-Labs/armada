// The sound names a notification carries, against the files the bundle ships.

import { readdirSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

import { soundOf } from "./telling";

describe("the tone", () => {
  it("names a file the bundle carries, one each", () => {
    // A name with no file behind it is a silent notification, and nothing else would say so.
    const shipped = readdirSync(join(__dirname, "../../sounds")).filter((file) => file.endsWith(".wav"));
    const named = [soundOf("blocked"), soundOf("waiting")];
    expect(new Set(named).size).toBe(2);
    expect(shipped.sort()).toEqual(named.map((name) => `${name}.wav`).sort());
  });
});
