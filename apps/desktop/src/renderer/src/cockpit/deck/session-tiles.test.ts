// Where a Session's Report starts its new Session: Armada's repository where Fleet serves one, else the reported Session's own.
// What Release names, where it may hand to, and when it is off.

import type { RepositorySummary } from "@armada/protocol";
import { expect, test } from "vitest";

import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";

import { handTargets, heldOf, nameOfHeldRow, offeredOf, releaseKeyOf, reportRoot } from "./session-tiles";

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

const planned = { work: { plan: { groups: [] } } } as unknown as Session;
const draftWith = (judge: boolean) => ({ commands: [], work: judge ? { judge: async () => false } : {} }) as unknown as SessionsDraft;

test("the Judge tile is offered for a Session with a plan, when Fleet can read one", () => {
  expect(offeredOf(planned, draftWith(true), "/code/shop").off.judge).toBeUndefined();
});

test("the Judge tile is off with no plan to judge against, or with no way to ask for a reading", () => {
  expect(offeredOf({} as Session, draftWith(true), "/code/shop").off.judge).toBe("No plan to judge against");
  expect(offeredOf(planned, draftWith(false), "/code/shop").off.judge).toBe("No plan to judge against");
});

test("the Judge tile is off with the reason a Session cannot be spoken to", () => {
  expect(offeredOf({ ...planned, dead: "ended" } as Session, draftWith(true), "/code/shop").off.judge).toBe("The Session ended");
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
      { kind: "job", id: "01JOB", number: 88, title: "Tidy the pool", state: "running", branch: "fleet/88" },
    ],
    needs: [{ path: "crates/ipc", what: "a new operation" }],
    rows: [],
    turn: { state: "idle" },
    ...over,
  }) as unknown as Session;

test("Release names each thing the Session holds, and leaves a merged pull request out", () => {
  expect(heldOf(held()).map((one) => [one.kind, one.name])).toEqual([
    ["slot", "slot-3"],
    ["branch", "fix/ledger"],
    ["pr", "#2430 Ledger rows"],
    ["job", "#88 Tidy the pool"],
    ["need", "crates/ipc"],
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
  expect(nameOfHeldRow(held(), { kind: "job", target: "01JOB" })).toBe("#88 Tidy the pool");
  expect(nameOfHeldRow(held(), { kind: "pr", target: "2430" })).toBe("#2430 Ledger rows");
  expect(nameOfHeldRow(held(), { kind: "slot", target: "3" })).toBe("slot-3");
});

test("Release is off where the Session holds nothing or nothing serves it", () => {
  const draft = { releaseHolds: async () => undefined, commands: [] } as unknown as SessionsDraft;
  expect(offeredOf(held(), draft, "/code").off.release).toBeUndefined();
  expect(offeredOf(held({ attachments: [], needs: [] }), draft, "/code").off.release).toBe("The Session holds nothing");
  expect(offeredOf(held(), { commands: [] } as unknown as SessionsDraft, "/code").off.release).toBe("Fleet does not offer this");
});
