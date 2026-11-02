// What the Timeline HUD lists under the stuck Jobs: one row for each Job or main that came to something since he last closed it.

import { expect, test } from "vitest";
import { GitMerge } from "lucide-react";

import { OUTCOME_KINDS } from "../../fleet-timeline";
import type { FleetEvent } from "../fleet-events";
import type { Subject } from "./ship";
import { movedRows, type HudRow } from "./timeline-rows";

const NOW = Date.parse("2026-11-02T12:00:00Z");
const ago = (minutes: number) => new Date(NOW - minutes * 60_000).toISOString();
const clock = (at: string) => at.slice(11, 16);
let ids = 0;
const job = (id: string, kind: FleetEvent["kind"], minutes: number, more: Partial<FleetEvent> = {}): FleetEvent => {
  ids += 1;
  return { id: String(ids), at: ago(minutes), kind, subject: { kind: "session", id, title: `Job ${id}` }, tone: "ok", says: "Done", repo: "armada", ...more };
};
const main = (kind: FleetEvent["kind"], minutes: number, repo = "armada"): FleetEvent => {
  ids += 1;
  return { id: String(ids), at: ago(minutes), kind, subject: { kind: "main" }, tone: kind === "main.red" ? "bad" : "ok", says: "Main", repo };
};
const merged = (id: string, number: number, minutes: number) => job(id, "pr.merged", minutes, { pull: { number, title: "" } });
const moved = (events: FleetEvent[], stuck: HudRow[] = [], subjects: Subject[] = [], since = NOW - 60 * 60_000) => movedRows(events, since, stuck, subjects, clock);

test("a Job only working has no row", () => {
  expect(moved([job("a", "session.turn", 5), job("a", "session.edited", 4), job("a", "pr.opened", 3)])).toEqual([]);
});

test("what came before he last closed it has no row", () => {
  expect(moved([job("a", "session.ended", 61)]).map((one) => one.title)).toEqual([]);
  expect(moved([job("a", "session.ended", 59)]).map((one) => one.title)).toEqual(["Job a"]);
});

test("a Job gets one row however much it came to, drawn as the newest, with every merge in its fact", () => {
  const rows = moved([merged("a", 11, 30), merged("a", 12, 20), job("a", "session.ended", 10)]);
  expect(rows).toHaveLength(1);
  expect(rows[0]).toMatchObject({ title: "Job a", said: "Finished", at: ago(10), fact: "#11 merged, #12 merged" });
});

test("the rows run newest first", () => {
  expect(moved([job("a", "session.ended", 30), merged("b", 7, 10), main("main.green", 20)]).map((one) => one.title)).toEqual(["Job b", "armada main", "Job a"]);
});

test("a Job already listed as stuck is not listed again", () => {
  const stuck: HudRow[] = [{ key: "session:a", title: "Job a", tone: "ask", Glyph: GitMerge, said: "Asking" }];
  expect(moved([job("a", "session.ended", 5), job("b", "session.ended", 6)], stuck).map((one) => one.title)).toEqual(["Job b"]);
});

test("each repository's main has its own row, naming it, with its red and green in the order they came", () => {
  const rows = moved([main("main.red", 30), main("main.green", 20), main("main.red", 10, "pocket")]);
  expect(rows.map((one) => [one.title, one.tone])).toEqual([
    ["pocket main", "bad"],
    ["armada main", "ok"],
  ]);
  expect(rows[1]?.fact).toBe(`red ${clock(ago(30))}, green ${clock(ago(20))}`);
});

test("a row opens the Job the ship still has, and nothing for one that is gone or for main", () => {
  const held = { key: "session:a" } as Subject;
  const rows = moved([job("a", "session.ended", 5), job("b", "session.ended", 6), main("main.green", 7)], [], [held]);
  expect(rows.map((one) => one.opens)).toEqual([held, undefined, undefined]);
});

test("every kind the list draws outlasts what Bridge keeps of the rest", () => {
  const drawn = (kind: FleetEvent["kind"]) => moved([kind.startsWith("main") ? main(kind, 1) : job("a", kind, 1)]).length > 0;
  const kinds: FleetEvent["kind"][] = ["session.started", "session.asked", "session.answered", "session.turn", "session.edited", "session.read", "session.ran", "session.ended", "pr.opened", "pr.checks", "pr.queued", "pr.merged", "main.red", "main.green"];
  expect(new Set(kinds.filter(drawn))).toEqual(OUTCOME_KINDS);
});
