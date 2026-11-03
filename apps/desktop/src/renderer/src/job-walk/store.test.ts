// Where each walk stands outside the tab that draws it, and the notes held on each Job: keyed by Job and walk, so another Job's walk of the same address is its own.

import { beforeEach, describe, expect, test } from "vitest";

import {
  approveWalk,
  arrivedBefore,
  bringWalkForward,
  editNote,
  forgetWalks,
  goToStep,
  heldOf,
  holdNote,
  isApproved,
  markSeen,
  pickTake,
  pressedWalks,
  removeNote,
  settlePressed,
  stateOf,
  takeHeld,
  workingOn,
  showAgain,
} from "./store";

const PAGE = "http://x/billing";

beforeEach(forgetWalks);

describe("a walk's state", () => {
  test("starts on the first take and step, unseen and not approved", () => {
    expect(stateOf("job", PAGE)).toMatchObject({
      take: 0,
      step: 0,
      seen: false,
      approved: false,
    });
  });

  test("is kept for its Job and its address alone", () => {
    pickTake("job", PAGE, 2);
    goToStep("job", PAGE, 3);
    markSeen("job", PAGE);
    approveWalk("job", PAGE);
    expect(stateOf("job", PAGE)).toMatchObject({
      take: 2,
      step: 3,
      seen: true,
      approved: true,
    });
    expect(isApproved("job", PAGE)).toBe(true);
    expect(isApproved("other", PAGE)).toBe(false);
    expect(isApproved("job", "http://x/settings")).toBe(false);
  });

  test("a walk reads still changing after its Job takes notes, and ready once it shows the page again", () => {
    workingOn("job", [
      PAGE,
    ]);
    expect(stateOf("job", PAGE).readiness).toBe("working");
    showAgain("job");
    expect(stateOf("job", PAGE).readiness).toBe("ready");
  });

  test("a tab is made once: the first ask says no and the rest say it was", () => {
    expect(arrivedBefore("job", PAGE)).toBe(false);
    expect(arrivedBefore("job", PAGE)).toBe(true);
    expect(arrivedBefore("other", PAGE)).toBe(false);
  });
});

describe("a walk pressed open", () => {
  test("waits until it is brought forward, and is told apart by Job and address", () => {
    bringWalkForward("job", PAGE);
    bringWalkForward("other", PAGE);
    expect(pressedWalks()).toEqual([
      {
        jobId: "job",
        walk: PAGE,
      },
      {
        jobId: "other",
        walk: PAGE,
      },
    ]);
    settlePressed("job", PAGE);
    expect(pressedWalks()).toEqual([
      {
        jobId: "other",
        walk: PAGE,
      },
    ]);
  });
});

describe("notes held on a Job", () => {
  test("are numbered in the order taken, edited and removed by id, and belong to the Job", () => {
    holdNote("job", "first", "Billing");
    holdNote("job", "second", "Billing");
    holdNote("other", "elsewhere", "Settings");
    expect(heldOf("job").map((note) => note.said)).toEqual([
      "first",
      "second",
    ]);
    const [one, two] = heldOf("job");
    expect(editNote("job", one!.id, "  first, again ")).toBe(true);
    expect(editNote("job", one!.id, "  ")).toBe(false);
    expect(removeNote("job", two!.id)).toBe(true);
    expect(removeNote("job", two!.id)).toBe(false);
    expect(takeHeld("job").map((note) => note.said)).toEqual([
      "first, again",
    ]);
    expect(takeHeld("job")).toEqual([]);
    expect(takeHeld("other").map((note) => note.said)).toEqual([
      "elsewhere",
    ]);
  });
});
