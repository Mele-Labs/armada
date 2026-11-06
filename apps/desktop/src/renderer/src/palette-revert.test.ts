// The palette offers Revert to the build before while one is kept, and choosing it asks main for the native confirm.

import { afterEach, expect, test, vi } from "vitest";

import { askRevertFleetBuild } from "./commands";
import { carryOut, REVERT_BUILD, revertRows, type PaletteHands } from "./palette";

afterEach(() => {
  vi.unstubAllGlobals();
});

test("the row is offered with a build before kept, and not without one", () => {
  expect(revertRows(true)).toEqual([
    {
      id: REVERT_BUILD,
      label: "Revert to the build before",
    },
  ]);
  expect(revertRows(false)).toEqual([]);
});

test("choosing the row asks for the revert, and nothing else", () => {
  const askRevert = vi.fn();
  const createMod = vi.fn();
  carryOut(
    {
      of: "board",
      id: REVERT_BUILD,
    },
    {
      askRevert,
      createMod,
    } as unknown as PaletteHands,
  );
  expect(askRevert).toHaveBeenCalledTimes(1);
  expect(createMod).not.toHaveBeenCalled();
});

test("the command calls the preload method", async () => {
  const ask = vi.fn(async () => {});
  vi.stubGlobal("window", {
    armada: {
      askRevertFleetBuild: ask,
    },
  });
  await askRevertFleetBuild();
  expect(ask).toHaveBeenCalledTimes(1);
});
