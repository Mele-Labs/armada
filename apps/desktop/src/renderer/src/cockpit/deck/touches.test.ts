// The directories a Session or a Job is changing, as the deck's battlefield places a drone over them: read from what
// Fleet observed, never from a table. Edits come from `session.edited` bursts (their `items`, paths newest first),
// rolled up to the two-level areas a repository is drawn in; a clash comes from the write-clash read and marks the
// areas of the paths it names.

import type { TimelineEvent, WriteClash } from "@armada/protocol";
import { expect, test } from "vitest";

import { areaOf, relativeTo } from "./areas";
import { clashesOf, touchesFromEvents, touchesOfFootprint } from "./touches";

const SLOT = "/Users/user/Development/armada/.armada/slots/slot-3";

let count = 0;
const burst = (kind: TimelineEvent["kind"], subject: TimelineEvent["subject"], items: string[] | undefined, minutes = 1): TimelineEvent => {
  count += 1;
  return { id: count, at: new Date(Date.parse("2026-10-14T12:00:00Z") - minutes * 60_000).toISOString(), kind, subject, tone: "run", says: `Edited ${items?.length ?? 0} files`, ...(items === undefined ? {} : { detail: items[0], items }), live: true };
};
const session = (id: string) => ({ kind: "session", id }) as const;
const job = (id: string) => ({ kind: "job", id }) as const;

const sources = {
  keyOf: (event: TimelineEvent) => (event.subject.kind === "session" || event.subject.kind === "job" ? `${event.subject.kind}:${event.subject.id}` : undefined),
  rootsOf: () => [SLOT],
};

test("an area is the first two directories of a path, and a file at the top is in none", () => {
  expect(areaOf("crates/fleet/src/a.rs")).toBe("crates/fleet");
  expect(areaOf("apps/desktop/src/renderer/src/cockpit/Cockpit.tsx")).toBe("apps/desktop");
  expect(areaOf("docs/README.md")).toBe("docs");
  expect(areaOf("Cargo.toml")).toBeUndefined();
  expect(areaOf("")).toBeUndefined();
});

test("a path under a checkout is made relative to it, in either spelling of the home directory", () => {
  expect(relativeTo(`${SLOT}/crates/fleet/a.rs`, [SLOT])).toBe("crates/fleet/a.rs");
  expect(relativeTo("~/Development/armada/.armada/slots/slot-3/crates/fleet/a.rs", [SLOT])).toBe("crates/fleet/a.rs");
  expect(relativeTo("docs/a.md", [SLOT])).toBe("docs/a.md");
  // Under no checkout it is somebody else's file and stays absolute, which no area claims.
  expect(relativeTo("/etc/hosts", [SLOT])).toBe("/etc/hosts");
});

test("touches come from observed edits: a Session's edited items, rolled up to areas and summed over its bursts", () => {
  const events = [
    burst("session.edited", session("s1"), [`${SLOT}/crates/fleet/src/a.rs`, `${SLOT}/crates/fleet/src/b.rs`, `${SLOT}/docs/concepts/x.md`], 5),
    burst("session.edited", session("s1"), ["~/Development/armada/.armada/slots/slot-3/crates/fleet/tests/c.rs"], 2),
    burst("session.edited", session("s2"), [`${SLOT}/apps/desktop/src/a.ts`]),
    // A read is not a touch, and neither is a file outside every checkout or at the top of one.
    burst("session.read", session("s1"), [`${SLOT}/crates/store/src/a.rs`]),
    burst("session.edited", session("s3"), ["/etc/hosts", `${SLOT}/Cargo.toml`]),
  ];
  const touched = touchesFromEvents(events, sources);

  expect(touched.get("session:s1")).toEqual([
    { path: "crates/fleet", w: 3 },
    { path: "docs/concepts", w: 1 },
  ]);
  expect(touched.get("session:s2")).toEqual([{ path: "apps/desktop", w: 1 }]);
  // Nothing observed is nothing touched: the drone is not given a directory.
  expect(touched.has("session:s3")).toBe(false);
});

test("a burst that kept no items touches nothing, and only the newest 25 of one count", () => {
  const many = Array.from({ length: 40 }, (_, at) => `${SLOT}/crates/fleet/src/f${at}.rs`);
  const touched = touchesFromEvents([burst("session.edited", session("s1"), undefined), burst("session.edited", session("s2"), many)], sources);

  expect(touched.has("session:s1")).toBe(false);
  expect(touched.get("session:s2")).toEqual([{ path: "crates/fleet", w: 25 }]);
});

test("a Job's edit event names one file, which is a touch; its footprint names them all", () => {
  const files = burst("job.files", job("j1"), undefined);
  const named = { ...files, detail: `${SLOT}/packages/protocol/src/x.ts` };
  expect(touchesFromEvents([named], sources).get("job:j1")).toEqual([{ path: "packages/protocol", w: 1 }]);
  expect(touchesOfFootprint([{ path: "crates/ipc/src/a.rs" }, { path: "crates/ipc/src/b.rs" }, { path: "docs/x.md" }, { path: "README.md" }])).toEqual([
    { path: "crates/ipc", w: 2 },
    { path: "docs", w: 1 },
  ]);
});

const clash = (one: WriteClash["one"], other: WriteClash["other"], paths: string[]): WriteClash => ({ one, other, paths, since: "2026-10-14T11:00:00Z" });
const holder = (kind: "job" | "session" | "main", id: string) => ({ kind, id, label: id });

test("a write clash marks the areas of the paths it names, for both holders", () => {
  const found = clashesOf([clash(holder("session", "s1"), holder("job", "j1"), ["crates/fleet/src/a.rs", "crates/fleet/src/b.rs", "docs/x.md"])], {
    keyOf: (kind, id) => (kind === "main" ? undefined : `${kind}:${id}`),
    rootOf: () => "/Users/user/armada",
  });

  expect(found).toEqual([
    { path: "crates/fleet", root: "/Users/user/armada", keys: ["session:s1", "job:j1"] },
    { path: "docs", root: "/Users/user/armada", keys: ["session:s1", "job:j1"] },
  ]);
});

test("a clash against main marks the agent's areas alone, and a holder the ship does not hold marks nothing", () => {
  const known = new Set(["session:s1"]);
  const sources = { keyOf: (kind: string, id: string) => (known.has(`${kind}:${id}`) ? `${kind}:${id}` : undefined), rootOf: () => "/r" };

  expect(clashesOf([clash(holder("session", "s1"), holder("main", "main"), ["apps/desktop/a.ts"])], sources)).toEqual([{ path: "apps/desktop", root: "/r", keys: ["session:s1"] }]);
  expect(clashesOf([clash(holder("job", "gone"), holder("session", "gone2"), ["apps/desktop/a.ts"])], sources)).toEqual([]);
});
