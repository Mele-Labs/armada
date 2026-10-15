// What a stranded bay offers on the deck is what the Bays page offers for it, in that order, and nothing where the window
// cannot send an act.

import type { Outcome, WorktreeSlot } from "@armada/protocol";
import { expect, test } from "vitest";

import { offeredOf, refusalOf } from "./rescue";

const stranded = (rescue?: Record<string, unknown>): WorktreeSlot => ({ manifest_id: "m", slot: 3, path: "/p", base: "main", warm: false, held: { state: "stranded" }, ...(rescue === undefined ? {} : { rescue }) }) as unknown as WorktreeSlot;
const NOTHING = { start: false, stop: false, pickUp: false, stash: false, scrap: false };

test("a stranded bay no Scout has read offers Rescue, and while it reads only Stop", () => {
  expect(offeredOf(stranded(), true)).toEqual({ ...NOTHING, start: true });
  expect(offeredOf(stranded({ state: "reading" }), true)).toEqual({ ...NOTHING, stop: true });
});

test("once read it offers Stash and Scrap, and Pick up only where there is work to continue", () => {
  expect(offeredOf(stranded({ state: "done", verdict: "unfinished" }), true)).toEqual({ ...NOTHING, pickUp: true, stash: true, scrap: true });
  expect(offeredOf(stranded({ state: "done", verdict: "scraps" }), true)).toEqual({ ...NOTHING, stash: true, scrap: true });
});

test("a bay that is not stranded, and a window that cannot send, offer nothing", () => {
  const leased = { manifest_id: "m", slot: 1, held: { state: "job", job_id: "j" } } as unknown as WorktreeSlot;
  expect(offeredOf(leased, true)).toEqual(NOTHING);
  expect(offeredOf(stranded(), false)).toEqual(NOTHING);
});

test("a refusal says what was not done, then Fleet's own words", () => {
  const refused = { ok: false, why: "refused", error: { message: "nothing to stash" } } as unknown as Outcome;
  expect(refusalOf("stash", refused)).toBe("Changes not stashed: nothing to stash");
});
