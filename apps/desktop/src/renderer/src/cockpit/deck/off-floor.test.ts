// The pad's lights from a Job's check rows, what the strip says each drone is doing, and the messages between Jobs, read only from what
// Fleet recorded.

import { expect, test } from "vitest";

import type { TimelineEvent } from "@armada/protocol";

import { FRESH_MS } from "./doing";
import { THINK_MS, fileNamed, lightsOf, mainNow, messageWord, messagesOf, quietAt, subagentNow, wordOf, type CheckRecord } from "./off-floor";

type NowInput = Parameters<typeof mainNow>[0];

const at = (s: number) => new Date(Date.UTC(2026, 9, 22, 12, 0, s)).toISOString();
const TEST = "apps/desktop/src/renderer/src/cockpit/deck/layout.test.ts";

test("a light is unlit before its check starts in this round, then running, passed or failed by the newest run since the last edit", () => {
  const first: CheckRecord[] = [
    { name: "fmt", at: at(1), state: "passed" },
    { name: "lint", at: at(2), state: "passed" },
    { name: "test", at: at(3), state: "running", target: TEST },
  ];
  expect(lightsOf(first, Date.parse(at(0))).map((one) => [one.name, one.state])).toEqual([["fmt", "ok"], ["lint", "ok"], ["test", "run"]]);
  const failed = [...first.slice(0, 2), { name: "test", at: at(3), state: "failed" as const, target: TEST }];
  expect(lightsOf(failed, undefined).at(-1)).toEqual({ name: "test", state: "bad", blame: TEST });
  // An edit at 10 seconds starts a new round: fmt runs again and the two not yet run are unlit, in the order they first ran.
  const again = [...failed, { name: "fmt", at: at(11), state: "running" as const }];
  expect(lightsOf(again, Date.parse(at(10)))).toEqual([{ name: "fmt", state: "run" }, { name: "lint", state: "idle" }, { name: "test", state: "idle" }]);
});

test("a failing check blames a file only where its record names one: a filter, a crate list or a passing check blames none", () => {
  expect(fileNamed(TEST)).toBe(TEST);
  expect(fileNamed("fleet, store")).toBeUndefined();
  expect(fileNamed("the_retry_test")).toBeUndefined();
  expect(fileNamed("crates/fleet")).toBeUndefined();
  expect(lightsOf([{ name: "test", at: at(1), state: "failed" }], undefined)).toEqual([{ name: "test", state: "bad" }]);
  expect(lightsOf([{ name: "test", at: at(1), state: "failed", target: "fleet, store" }], undefined)).toEqual([{ name: "test", state: "bad" }]);
  expect(lightsOf([{ name: "test", at: at(1), state: "passed", target: TEST }], undefined)).toEqual([{ name: "test", state: "ok" }]);
});

const idle: NowInput = { working: false, walk: undefined, checking: false, quiet: false, call: undefined, burst: undefined, file: undefined, owed: false, done: false };
const busy = { ...idle, working: true };

test("the strip says what each drone is doing in plain words, a walk first, then checks, thinking, a call, a read or a write", () => {
  const said = (input: Partial<NowInput>) => wordOf(mainNow({ ...idle, ...input }));
  expect(said({ ...busy, burst: { act: "write", file: "apps/desktop/src/layout.ts" } })).toBe("Writing layout.ts");
  expect(said({ ...busy, burst: { act: "read", file: "packages/protocol/src/wire.ts" } })).toBe("Reading wire.ts");
  expect(said({ ...busy, burst: { act: "read", file: "wire.ts" }, checking: true })).toBe("Running checks");
  expect(said({ ...busy, checking: true, walk: "The pad on the Field" })).toBe("Walking the app");
  expect(said({ ...busy, quiet: true, burst: { act: "write", file: "layout.ts" } })).toBe("Thinking");
  expect(said({ ...busy, call: { service: "forge", name: "the forge", line: "pr create" }, burst: { act: "write", file: "layout.ts" } })).toBe("Calling Forge");
  expect(said({ ...busy, file: "apps/desktop/src/layout.ts" })).toBe("Writing layout.ts");
  expect(said({ owed: true })).toBe("Waiting on you");
  expect(said({ done: true })).toBe("Done");
  expect(said({})).toBe("Idle");
  // A quiet spell is no thinking once the turn is over.
  expect(said({ quiet: true, owed: true })).toBe("Waiting on you");
  expect(wordOf(subagentNow(false, "packages/protocol/src/events.ts"))).toBe("Writing events.ts");
  expect(wordOf(subagentNow(false, undefined))).toBe("Working");
  expect(wordOf(subagentNow(true, "packages/protocol/src/events.ts"))).toBe("Done");
});

test("a fresh write takes the drone to its file past a walk still open, and the walk has it back once the write is not fresh", () => {
  const said = (input: Partial<NowInput>) => wordOf(mainNow({ ...idle, ...input, walk: "The pad on the Field" }));
  expect(said({ ...busy, burst: { act: "write", file: "apps/desktop/src/layout.ts" } })).toBe("Writing layout.ts");
  expect(said({ ...busy, burst: { act: "read", file: "apps/desktop/src/layout.ts" } })).toBe("Walking the app");
  expect(said({ ...busy, quiet: true, burst: { act: "write", file: "apps/desktop/src/layout.ts" } })).toBe("Walking the app");
  expect(said({ ...busy, file: "apps/desktop/src/layout.ts" })).toBe("Walking the app");
  expect(said({ burst: { act: "write", file: "apps/desktop/src/layout.ts" } })).toBe("Walking the app");
});

test("a working turn is not thinking five seconds after its last tool call, and is at twelve", () => {
  const last = Date.parse(at(0));
  expect(THINK_MS).toBe(12_000);
  expect(quietAt(last, last + 5_000)).toBe(false);
  expect(quietAt(last, last + 11_999)).toBe(false);
  expect(quietAt(last, last + 12_000)).toBe(true);
  expect(quietAt(undefined, last + 60_000)).toBe(false);
  const said = (quiet: boolean) => wordOf(mainNow({ ...busy, quiet, burst: { act: "write", file: "layout.ts" } }));
  expect(said(quietAt(last, last + 5_000))).toBe("Writing layout.ts");
  expect(said(quietAt(last, last + 12_000))).toBe("Thinking");
});

const TITLES: Record<string, string> = { a: "Rotate the audit log", b: "Rebuild the store index", c: "Retry the slot lease", d: "Pin the clock" };
const message = (id: number, from: string, to: string, ms: number, text: string): TimelineEvent => ({
  id,
  at: new Date(ms).toISOString(),
  kind: "session.messaged",
  subject: { kind: "session", id: from },
  title: TITLES[from]!,
  tone: "run",
  says: `Messaged: ${text}`,
  to,
});

test("a message is read only while its entry is fresh, and is gone once it is stale", () => {
  const sent = Date.UTC(2026, 9, 22, 12, 0, 0);
  const events = [message(1, "a", "b", sent, "Is the index rebuilt?")];
  expect(messagesOf(events, "a", sent + 1000)).toEqual([{ other: "b", sent: true, quote: "Is the index rebuilt?", at: sent }]);
  expect(messagesOf(events, "a", sent + FRESH_MS - 1)).toHaveLength(1);
  expect(messagesOf(events, "a", sent + FRESH_MS)).toEqual([]);
  // Not yet, at a clock before it.
  expect(messagesOf(events, "a", sent - 1)).toEqual([]);
});

test("a message runs from the sender to the receiver: sent by this Job, or sent to it by the other, one for each other Job from its newest", () => {
  const t = Date.UTC(2026, 9, 22, 12, 0, 0);
  const events = [message(1, "a", "b", t, "Is the index rebuilt?"), message(2, "b", "a", t + 5000, "Yes, at startup"), message(3, "c", "d", t + 6000, "Not ours"), message(4, "c", "a", t + 2000, "Lease is retried")];
  expect(messagesOf(events, "a", t + 10_000)).toEqual([
    { other: "b", sent: false, quote: "Yes, at startup", at: t + 5000, title: "Rebuild the store index" },
    { other: "c", sent: false, quote: "Lease is retried", at: t + 2000, title: "Retry the slot lease" },
  ]);
  expect(messagesOf(events, "b", t + 10_000).map((one) => [one.other, one.sent])).toEqual([["a", true]]);
  expect(messageWord({ sent: true }, "Rebuild the store index")).toBe("Messaging Rebuild the store index");
  expect(messageWord({ sent: false }, "Rotate the audit log")).toBe("Messaged by Rotate the audit log");
});
