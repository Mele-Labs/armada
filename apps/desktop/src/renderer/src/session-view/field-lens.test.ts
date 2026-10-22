import { expect, test } from "vitest";

import { lensOf, writersOf } from "../cockpit/deck/lens";
import { lensFrom, writingOf } from "./field-lens";
import type { DroneIn } from "../cockpit/deck/layout";

const change = (path: string) => lensOf({ ok: true, stamp: path, diff: `diff --git a/${path} b/${path}\n--- a/${path}\n+++ b/${path}\n@@ -1,1 +1,2 @@\n a\n+b\n` });
const kept = new Map([["a.ts", change("a.ts")], ["b.ts", change("b.ts")], ["c.ts", change("c.ts")]]);

test("the card is on the newest writer's file, every writer's file lifts, and the files written before stand behind as tabs", () => {
  const writers = [...writersOf(writersOf(new Map(), [{ key: "main", path: "a.ts" }], 1), [{ key: "main", path: "a.ts" }, { key: "sub", path: "b.ts" }], 2).values()];
  const bursts = [{ at: 1, act: "write" as const, paths: ["c.ts"] }, { at: 2, act: "read" as const, paths: ["d.ts"] }, { at: 3, act: "write" as const, paths: ["b.ts"] }];
  const { slabs, card } = lensFrom({ writers, pinned: undefined, kept, newest: new Map(), refused: new Set(), bursts });
  expect(slabs.map((one) => [one.path, one.card])).toEqual([["a.ts", false], ["b.ts", true]]);
  expect(card?.path).toBe("b.ts");
  expect(card?.tabs.map((one) => one.path)).toEqual(["c.ts"]);
  expect(lensFrom({ writers, pinned: "main", kept, newest: new Map(), refused: new Set(), bursts }).card?.path).toBe("a.ts");
});

test("a refused file draws no lens, and a file not answered yet lifts nothing", () => {
  const writers = [...writersOf(new Map(), [{ key: "main", path: "a.ts" }, { key: "sub", path: "x.ts" }], 1).values()];
  const { slabs, card } = lensFrom({ writers, pinned: undefined, kept, newest: new Map(), refused: new Set(["a.ts"]), bursts: [] });
  expect(slabs).toEqual([]);
  expect(card).toBeUndefined();
});

test("only the Job's own drones writing on the floor lift a file", () => {
  const drone = (key: string, more: Partial<DroneIn>): DroneIn => ({ key, kind: "session", title: key, root: "/r", said: "", hue: "calm", spin: false, hover: true, ping: false, stuck: false, flare: false, working: true, touches: [], shown: true, ...more });
  expect(writingOf([drone("a", { doing: "write", file: "a.ts" }), drone("b", { doing: "read", file: "b.ts" }), drone("c", { doing: "write", file: "c.ts", ghost: true }), drone("d", { doing: "write", file: "d.ts", off: "think" })])).toEqual([{ key: "a", path: "a.ts" }]);
});
