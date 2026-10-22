import type { TimelineEvent } from "@armada/protocol";
import { expect, test } from "vitest";

import { GLOW_MS, burstsOf, droneAt, glowAt, routeAt } from "./afterglow";

const T0 = Date.parse("2026-10-22T09:00:00Z");
const MIN = 60_000;
let id = 0;
const burst = (minutes: number, kind: "session.read" | "session.edited", items: string[], session = "s1"): TimelineEvent => ({
  id: ++id,
  at: new Date(T0 + minutes * MIN).toISOString(),
  kind,
  subject: { kind: "session", id: session },
  tone: "run",
  says: "Read",
  detail: items[0]!,
  items,
});

const ROOTS = ["/Users/user/armada/.armada/slots/slot-1", "/Users/user/armada"];

test("a write glows for ten minutes and a read for five, fading linearly, and the stronger stands on a file both touched", () => {
  const bursts = burstsOf([burst(0, "session.edited", ["crates/a.rs"]), burst(0, "session.read", ["crates/b.rs"])], "s1", ROOTS);
  expect(glowAt(bursts, T0).get("crates/a.rs")).toEqual({ act: "write", v: 1 });
  expect(glowAt(bursts, T0 + 5 * MIN).get("crates/a.rs")).toEqual({ act: "write", v: 0.5 });
  expect(glowAt(bursts, T0 + 2.5 * MIN).get("crates/b.rs")).toEqual({ act: "read", v: 0.5 });
  expect(glowAt(bursts, T0 + GLOW_MS.read).has("crates/b.rs")).toBe(false);
  expect(glowAt(bursts, T0 + GLOW_MS.write).size).toBe(0);
  // Before the burst nothing glows.
  expect(glowAt(bursts, T0 - 1).size).toBe(0);

  const both = burstsOf([burst(0, "session.edited", ["crates/a.rs"]), burst(4, "session.read", ["crates/a.rs"])], "s1", ROOTS);
  expect(glowAt(both, T0 + 4 * MIN).get("crates/a.rs")).toEqual({ act: "read", v: 1 });
  expect(glowAt(both, T0 + 8 * MIN).get("crates/a.rs")).toEqual({ act: "write", v: 0.2 });
});

test("a glow moves in fortieths, so a clock step that moves nothing visible hands over the same map", () => {
  const bursts = burstsOf([burst(0, "session.edited", ["crates/a.rs"])], "s1", ROOTS);
  expect(glowAt(bursts, T0 + 1000)).toEqual(glowAt(bursts, T0 + 5000));
});

test("only this Job's reads and writes count, spelled as the repository spells them", () => {
  const bursts = burstsOf(
    [burst(0, "session.edited", ["/Users/user/armada/.armada/slots/slot-1/crates/a.rs"]), burst(1, "session.edited", ["crates/z.rs"], "s2"), { ...burst(2, "session.read", ["x"]), kind: "session.ran" }],
    "s1",
    ROOTS,
  );
  expect(bursts).toEqual([{ at: T0, act: "write", paths: ["crates/a.rs"] }]);
});

test("a file outside the Job's checkouts lights nothing and moves neither the route nor the drone", () => {
  // As Fleet wrote them for a Job that read a plugin's notes after editing: shortened under the home directory.
  const bursts = burstsOf(
    [
      burst(0, "session.edited", ["~/armada/.armada/slots/slot-1/packages/components/src/a.tsx"]),
      burst(1, "session.read", ["~/.claude/plugins/cache/impeccable/reference/craft-floor.md", "~/.claude/plugins/cache/impeccable/reference/operate.md"]),
      burst(2, "session.read", ["/private/tmp/notes.txt", "~/armada/.armada/slots/slot-1/packages/components/src/b.tsx"]),
    ],
    "s1",
    ROOTS,
  );
  expect([...glowAt(bursts, T0 + 2 * MIN).keys()]).toEqual(["packages/components/src/a.tsx", "packages/components/src/b.tsx"]);
  expect(droneAt(bursts, T0 + 1.5 * MIN)).toEqual({ path: "packages/components/src/a.tsx", act: "write" });
  expect(droneAt(bursts, T0 + 2 * MIN)).toEqual({ path: "packages/components/src/b.tsx", act: "read" });
  expect(routeAt(bursts, T0 + 2 * MIN).map((one) => one.path)).toEqual(["packages/components/src/a.tsx", "packages/components/src/b.tsx"]);
});

test("the route numbers each stop from the first, folds a repeat, keeps the newest, and the drone stands at the last before the slider", () => {
  const bursts = burstsOf(
    [burst(0, "session.read", ["a"]), burst(1, "session.read", ["a"]), burst(2, "session.edited", ["b"]), burst(3, "session.read", ["c"]), burst(4, "session.edited", ["d"])],
    "s1",
    ROOTS,
  );
  expect(routeAt(bursts, T0 + 3 * MIN)).toEqual([
    { n: 1, path: "a", act: "read" },
    { n: 2, path: "b", act: "write" },
    { n: 3, path: "c", act: "read" },
  ]);
  expect(routeAt(bursts, T0 + 10 * MIN, 2).map((one) => one.n)).toEqual([3, 4]);
  expect(droneAt(bursts, T0 + 2.5 * MIN)).toEqual({ path: "b", act: "write" });
  expect(droneAt(bursts, T0 - MIN)).toBeUndefined();
});
