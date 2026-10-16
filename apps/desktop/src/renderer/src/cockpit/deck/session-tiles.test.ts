// Where a Session's Report starts its new Session: Armada's repository where Fleet serves one, else the reported Session's own.

import type { RepositorySummary } from "@armada/protocol";
import { expect, test } from "vitest";

import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";

import { offeredOf, reportRoot } from "./session-tiles";

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
