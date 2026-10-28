// Where a Session's Report starts its new Session: Armada's repository where Fleet serves one, else the reported Session's own.
// What Release names, where it may hand to, and when it is off.

import type { RepositorySummary } from "@armada/protocol";
import { expect, test } from "vitest";

import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";

import { LAND_OFF_TEXT, LAND_ON_TEXT, SESSION_TILES, handTargets, heldKey, heldOf, heldOfAttachment, nameOfHeldRow, offeredOf, releaseKeyOf, reportRoot, tileKey } from "./session-tiles";

const served = (root: string, id?: string, repository = id): RepositorySummary =>
  ({ root, records_root: `${root}/.records`, ...(id === undefined ? {} : { manifest: { id, repository } }) }) as unknown as RepositorySummary;

test("a report starts in Armada's repository when Fleet serves it beside others", () => {
  const repositories = [served("/code/shop", "shop"), served("/code/armada", "armada")];
  expect(reportRoot(repositories, "/code/shop")).toBe("/code/armada");
});

test("Armada's repository is found by its Manifest's id or its directory, in any case", () => {
  expect(reportRoot([served("/code/a", "Armada", "x")], "/code/shop")).toBe("/code/a");
  expect(reportRoot([served("/code/b", "m1", "armada")], "/code/shop")).toBe("/code/b");
});

test("without Armada's repository a report starts in the reported Session's own", () => {
  expect(reportRoot([served("/code/shop", "shop"), served("/code/bare")], "/code/shop")).toBe("/code/shop");
  expect(reportRoot([], undefined)).toBeUndefined();
});

const planned = { attachments: [], rows: [], work: { plan: { groups: [] } } } as unknown as Session;
const bare = { attachments: [], rows: [] } as unknown as Session;
const draftWith = (judge: boolean) => ({ commands: [], work: judge ? { judge: async () => false } : {} }) as unknown as SessionsDraft;

test("the Judge tile is offered for a Session with a plan, when Fleet can read one", () => {
  expect(offeredOf(planned, draftWith(true), "/code/shop").off.judge).toBeUndefined();
});

test("the Judge tile is off with no plan to judge against, or with no way to ask for a reading", () => {
  expect(offeredOf(bare, draftWith(true), "/code/shop").off.judge).toBe("No plan to judge against");
  expect(offeredOf(planned, draftWith(false), "/code/shop").off.judge).toBe("No plan to judge against");
});

test("the Judge tile is off with the reason a Session cannot be spoken to", () => {
  expect(offeredOf({ ...planned, dead: "ended" } as Session, draftWith(true), "/code/shop").off.judge).toBe("The Job ended");
});

const held = (over: Partial<Session> = {}): Session =>
  ({
    id: "s1",
    title: "Fix the ledger",
    manifestId: "armada",
    attachments: [
      { kind: "slot", slot: 3 },
      { kind: "branch", name: "fix/ledger", slot: 3 },
      { kind: "pull_request", number: 2430, title: "Ledger rows", branch: "fix/ledger", address: "", checks: { state: "none" }, state: "open", auto: false },
      { kind: "pull_request", number: 2401, title: "Old one", branch: "x", address: "", checks: { state: "none" }, state: "merged", auto: false },
    ],
    rows: [],
    turn: { state: "idle" },
    ...over,
  }) as unknown as Session;

test("Release names each thing the Session holds, and leaves a merged pull request out", () => {
  expect(heldOf(held()).map((one) => [one.kind, one.name])).toEqual([
    ["slot", "slot-3"],
    ["branch", "fix/ledger"],
    ["pr", "#2430 Ledger rows"],
  ]);
});

test("a hand goes to a live Session of the same repository, and the key names it", () => {
  const all = [held(), held({ id: "s2", title: "Takes over" }), held({ id: "s3", manifestId: "shop" }), held({ id: "s4", dead: "ended" })];
  const targets = handTargets(held(), all);
  expect(targets.map((one) => one.id)).toEqual(["s2"]);
  expect(releaseKeyOf(undefined)).toBe("Release all");
  expect(releaseKeyOf(undefined, false)).toBe("Release");
  expect(releaseKeyOf(targets[0])).toBe("Hand to Takes over");
});

test("Fleet's rows are named as the confirmation named them", () => {
  expect(nameOfHeldRow(held(), { kind: "pr", target: "2430" })).toBe("#2430 Ledger rows");
  expect(nameOfHeldRow(held(), { kind: "slot", target: "3" })).toBe("slot-3");
});

test("Release is off where the Session holds nothing or nothing serves it", () => {
  const draft = { releaseHolds: async () => undefined, commands: [] } as unknown as SessionsDraft;
  expect(offeredOf(held(), draft, "/code").off.release).toBeUndefined();
  expect(offeredOf(held({ attachments: [] }), draft, "/code").off.release).toBe("The Job holds nothing");
  expect(offeredOf(held(), { commands: [] } as unknown as SessionsDraft, "/code").off.release).toBe("Fleet does not offer this");
});

test("a ledger row's Release names the row as the tile's confirmation does, and a spent pull request or an artifact has none", () => {
  expect(heldOfAttachment({ kind: "slot", slot: 7 } as never)).toEqual({ kind: "slot", target: "7", name: "slot-7", said: "Slot" });
  expect(heldKey({ kind: "branch", target: "fix/a" })).toBe("branch:fix/a");
  expect(heldOfAttachment({ kind: "pull_request", number: 4, title: "A", state: "merged" } as never)).toBeUndefined();
  expect(heldOfAttachment({ kind: "artifact", id: "x", title: "X", form: "page" } as never)).toBeUndefined();
});

test("the tiles take Option and Shift with the digits 1 to 9 and then 0, and the last has none", () => {
  expect(SESSION_TILES.map((_, at) => tileKey(at))).toEqual(["⌥⇧1", "⌥⇧2", "⌥⇧3", "⌥⇧4", "⌥⇧5", "⌥⇧6", "⌥⇧7", "⌥⇧8", "⌥⇧9", "⌥⇧0", undefined, undefined, undefined, undefined, undefined]);
  expect(SESSION_TILES.slice(7).map((one) => one.id)).toEqual(["artifacts", "subagents", "checks", "slot", "requests", "terminal", "browser", "pause"]);
});

test("Terminal is off with the reason where the Job ended or Fleet serves no shell", () => {
  const shells = { commands: [], work: {}, terminalRuns: true } as unknown as SessionsDraft;
  expect(offeredOf(bare, shells, "/code/shop").off.terminal).toBeUndefined();
  expect(offeredOf({ ...bare, dead: "ended" } as Session, shells, "/code/shop").off.terminal).toBe("The Job ended");
  expect(offeredOf(bare, draftWith(false), "/code/shop").off.terminal).toBe("Fleet does not offer this");
});

test("Auto land is off where Fleet does not offer it, and offered where the draft can press it", () => {
  const without = { commands: [], work: {} } as unknown as SessionsDraft;
  const with_ = { commands: [], work: {}, autoLand: async () => true } as unknown as SessionsDraft;
  expect(offeredOf(bare, without, "/code").off.land).toBe("Fleet does not offer this");
  expect(offeredOf(bare, with_, "/code").off.land).toBeUndefined();
  expect(offeredOf({ ...bare, dead: "ended" } as Session, with_, "/code").off.land).toBe("The Job ended");
});

test("the words sent with Auto land say Fleet merges, and name no merge for the agent to do", () => {
  expect(LAND_ON_TEXT).toContain("Fleet turns on auto-merge");
  expect(LAND_ON_TEXT).toContain("Do not merge it yourself");
  expect(LAND_OFF_TEXT).toContain("Fleet turns auto-merge off");
  expect(`${LAND_ON_TEXT}${LAND_OFF_TEXT}`).not.toMatch(/[\u2013\u2014]| - /);
});

test("Subagents and Checks are off with nothing to list, and Bays is off where the Session holds no bay", () => {
  const draft = draftWith(false);
  expect(offeredOf(bare, draft, "/code").off).toMatchObject({ subagents: "The Job has no subagents", checks: "The Job has run no Checks", slot: "The Job holds no bay" });
  const made = { attachments: [{ kind: "subagent", id: "sa", task: "Read", state: "running" }], rows: [{ id: "c", at: "13:00", kind: "check", name: "fleet", run: 1, state: "passed" }] } as unknown as Session;
  const off = offeredOf(made, draft, "/code", true).off;
  expect(off.subagents).toBeUndefined();
  expect(off.checks).toBeUndefined();
  expect(off.slot).toBeUndefined();
});
