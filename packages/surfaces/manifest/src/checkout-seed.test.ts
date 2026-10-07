// What the Manifest surface's Setup line says about `setup.seed`. #1064. `seedSaid` itself is
// tested beside `seed.ts`, which the run sheet reads too.

import { expect, it } from "vitest";

import type { CheckoutRunSheet, DeclaredSeed } from "@armada/protocol";
import { checkoutGroupsOf } from "./checkout-runs";

const DECLARED: DeclaredSeed = {
  paths: ["target"],
  warmed_by: ["warm_build", "warm_tests"],
  warmth: { state: "warm", commit: "a787ffc2c1d0aa00000000000000000000000000" },
};

it("is the Manifest surface's Setup line, and absent where none is declared", () => {
  const sheet: CheckoutRunSheet = { setup: [], checks: [], commands: [] };
  expect(checkoutGroupsOf({ ...sheet, seed: DECLARED }).find((group) => group.kind === "setup")?.says).toContain(
    "warmed by warm_build",
  );
  expect(checkoutGroupsOf(sheet).some((group) => "says" in group)).toBe(false);
});
