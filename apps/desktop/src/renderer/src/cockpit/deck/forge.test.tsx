// What main's light says of the forge: how old the reading is, and which repositories it has stopped answering for.

import { expect, test } from "vitest";
import type { MergeLineView } from "@armada/screens";

import { DEFAULT_TIME_FORMAT, timeSaid } from "@armada/components";

import { forgeOf, quietSaid, readSaid } from "./forge";

const at = (hour: number, minute: number) => new Date(2026, 9, 16, hour, minute).toISOString();
const view = (root: string, hub: { readAt?: string; quietSince?: string }): MergeLineView => ({ root, line: [], landed: [], sentBack: [], hub: { pulls: [], ...hub } });
const named = (root: string) => root.slice(1);

test("the age reads in seconds, then minutes, then hours and minutes", () => {
  expect(readSaid(2000)).toBe("Read just now");
  expect(readSaid(40_000)).toBe("Read 40s ago");
  expect(readSaid(3 * 60_000 + 20_000)).toBe("Read 3m ago");
  expect(readSaid(125 * 60_000)).toBe("Read 2h 05m ago");
});

test("one repository says since when, with no name", () => {
  const forge = forgeOf([view("/armada", { quietSince: at(18, 40) })], named);
  expect(quietSaid(forge, false, DEFAULT_TIME_FORMAT)).toBe(`GitHub not answering since ${timeSaid(at(18, 40), DEFAULT_TIME_FORMAT)}`);
});

test("several repositories name each one that is quiet, and only those", () => {
  const views = [view("/armada", { quietSince: at(18, 40) }), view("/pocket", { quietSince: at(18, 42) }), view("/notes", {})];
  expect(quietSaid(forgeOf(views, named), true, DEFAULT_TIME_FORMAT)).toBe(`GitHub not answering for armada since ${timeSaid(at(18, 40), DEFAULT_TIME_FORMAT)}, pocket since ${timeSaid(at(18, 42), DEFAULT_TIME_FORMAT)}`);
  expect(quietSaid(forgeOf([views[0]!, views[2]!], named), true, DEFAULT_TIME_FORMAT)).toBe(`GitHub not answering for armada since ${timeSaid(at(18, 40), DEFAULT_TIME_FORMAT)}`);
});

test("nothing quiet says nothing", () => {
  expect(quietSaid(forgeOf([view("/armada", { readAt: at(18, 40) })], named), false)).toBeUndefined();
});

test("the light speaks for the oldest reading among the repositories", () => {
  const views = [view("/armada", { readAt: at(18, 41) }), view("/pocket", { readAt: at(18, 30) }), view("/notes", {})];
  expect(forgeOf(views, named).readAt).toBe(new Date(at(18, 30)).getTime());
  expect(forgeOf([view("/notes", {})], named).readAt).toBeUndefined();
});
