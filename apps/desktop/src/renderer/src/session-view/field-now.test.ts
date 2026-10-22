// What the strip over a Job's Field says of each drone, and the subagents that lift off and fly back as the ledger starts and ends them.

import { expect, test } from "vitest";
import type { Session } from "@armada/screens/src/draft/sessions";

import type { DroneIn } from "../cockpit/deck/layout";
import { liftsAndReturns, placedNow, stripOf } from "./field-now";

const drone = (key: string, extra: Partial<DroneIn> = {}): DroneIn => ({ key, kind: "session", title: key, root: "/r", said: "Working", hue: "calm", spin: true, hover: true, ping: false, stuck: false, flare: false, working: true, touches: [], shown: true, ...extra });

test("a subagent that starts lifts off, and one that ends flies back and is kept, still, until it has", () => {
  const a = drone("session:j/a", { parent: "session:j", doing: "write" });
  const b = drone("session:j/b", { parent: "session:j" });
  const { added, gone } = liftsAndReturns(new Map([[a.key, a]]), new Map([[b.key, b]]), 5);
  expect(added.map((one) => [one.kind, one.key])).toEqual([["launch", b.key], ["dock", a.key]]);
  expect(gone).toHaveLength(1);
  expect(gone[0]).toMatchObject({ key: a.key, parent: "session:j", spin: false, working: false });
  expect(gone[0]!.doing).toBeUndefined();
  expect(liftsAndReturns(new Map([[a.key, a]]), new Map([[a.key, a]]), 6).added).toEqual([]);
});

test("the strip lists the Job's drone first, then each subagent by its task in the order it started, a finished one kept and marked done", () => {
  const session = {
    id: "j",
    title: "Draw the pad",
    attachments: [
      { kind: "subagent", id: "a", task: "Check the wire types", state: "done" },
      { kind: "subagent", id: "b", task: "Read the layout", state: "running", call: "toolu_b" },
    ],
  } as unknown as Session;
  const strip = stripOf(session, { kind: "checks" }, [{ id: "toolu_b", agent: "Explore", path: "apps/desktop", doing: "write", file: "apps/desktop/src/layout.ts" }]);
  expect(strip).toEqual([
    { key: "session:j", name: "Draw the pad", word: "Running checks", done: false },
    { key: "session:j/a", name: "Check the wire types", word: "Done", done: true },
    { key: "session:j/b", name: "Read the layout", word: "Writing layout.ts", done: false },
  ]);
});

test("the Job's drone leaves the floor for the pad, rises to think with no action, and waits on you amber", () => {
  const main = drone("session:j", { doing: "write", via: "a.ts", file: "a.ts", touches: [{ path: "a.ts", w: 4 }] });
  expect(placedNow(main, { kind: "checks" })).toMatchObject({ off: "pad", touches: [] });
  expect(placedNow(main, { kind: "walk", name: "x" }).off).toBe("walk");
  const thinking = placedNow(main, { kind: "think" });
  expect(thinking).toMatchObject({ off: "think", file: "a.ts" });
  expect(thinking.doing).toBeUndefined();
  expect(placedNow(main, { kind: "read", file: "b.ts" })).toMatchObject({ doing: "read", file: "b.ts" });
  expect(placedNow(main, { kind: "call", service: "forge", name: "Forge" })).toMatchObject({ calling: "forge" });
  expect(placedNow(main, { kind: "ask" })).toMatchObject({ hue: "ask", ping: true });
});
