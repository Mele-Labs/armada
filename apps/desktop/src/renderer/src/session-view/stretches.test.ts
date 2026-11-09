// The plan read as gates: a run of groups up to the next gate is one stretch, the first whose gate is open is "now", and Held marks only the open gate the Job waits on.

import { expect, test } from "vitest";
import type { WorkGroup } from "@armada/screens/src/draft/sessions";

import { gated, stretchesOf } from "./stretches";

const group = (id: string, extra: Partial<WorkGroup> = {}): WorkGroup => ({
  id,
  agent: "implementer",
  title: id.toUpperCase(),
  paths: [],
  tasks: [],
  ...extra,
});
const closed = {
  at: "2026-11-09T10:00:00Z",
  fact: "Walk approved",
};

test("groups with no gate between them are one stretch, named for all of them", () => {
  const [only] = stretchesOf([
    group("a"),
    group("b", {
      gate: "walk_approved",
    }),
  ]);
  expect(only!.name).toBe("A · B");
  expect(only!.key).toBe("a+b");
  expect(only!.groups.map((one) => one.id)).toEqual([
    "a",
    "b",
  ]);
  expect(only!.gate?.kind).toBe("walk_approved");
});

test("the first stretch whose gate is not closed is now, later ones next, closed ones done", () => {
  const stretches = stretchesOf([
    group("a", {
      gate: "plan_approved",
      gateClosed: closed,
    }),
    group("b", {
      gate: "walk_approved",
    }),
    group("c", {
      gate: "merged",
    }),
  ]);
  expect(stretches.map((one) => one.state)).toEqual([
    "done",
    "now",
    "next",
  ]);
});

test("a closed gate after an open one is still done, and the open one stays now", () => {
  const stretches = stretchesOf([
    group("a", {
      gate: "walk_approved",
    }),
    group("b", {
      gate: "merged",
      gateClosed: closed,
    }),
  ]);
  expect(stretches.map((one) => one.state)).toEqual([
    "now",
    "done",
  ]);
});

test("a trailing run with no gate is its own stretch with no gate", () => {
  const stretches = stretchesOf([
    group("a", {
      gate: "plan_approved",
      gateClosed: closed,
    }),
    group("b"),
    group("c"),
  ]);
  expect(stretches).toHaveLength(2);
  expect(stretches[1]!.key).toBe("b+c");
  expect(stretches[1]!.gate).toBeUndefined();
  expect(stretches[1]!.state).toBe("now");
});

test("a stretch carries its last group's gate, closing fact and checks", () => {
  const checks = {
    commit: "abc123",
    state: "red" as const,
    failed: [
      "t1",
    ],
  };
  const [done, open] = stretchesOf([
    group("a", {
      gate: "plan_approved",
      gateClosed: closed,
    }),
    group("b", {
      gate: "checks_green",
      gateChecks: checks,
    }),
  ]);
  expect(done!.gate).toMatchObject({
    kind: "plan_approved",
    closed,
  });
  expect(open!.gate).toMatchObject({
    kind: "checks_green",
    closed: undefined,
    checks,
  });
});

test("held lands on the stretch whose last group matches, while its gate is open", () => {
  const stretches = stretchesOf(
    [
      group("a", {
        gate: "plan_approved",
        gateClosed: closed,
      }),
      group("b", {
        gate: "walk_approved",
      }),
    ],
    {
      group: "b",
      since: "10:05",
    },
  );
  expect(stretches[0]!.gate?.held).toBeUndefined();
  expect(stretches[1]!.gate?.held).toBe("10:05");
});

test("held does not mark a stretch whose gate is closed, or a group that is not its last", () => {
  const closedHeld = stretchesOf(
    [
      group("a", {
        gate: "walk_approved",
        gateClosed: closed,
      }),
    ],
    {
      group: "a",
      since: "10:05",
    },
  );
  expect(closedHeld[0]!.gate?.held).toBeUndefined();

  const notLast = stretchesOf(
    [
      group("a"),
      group("b", {
        gate: "walk_approved",
      }),
    ],
    {
      group: "a",
      since: "10:05",
    },
  );
  expect(notLast[0]!.gate?.held).toBeUndefined();
});

test("gated is true where any group carries a gate, false where none does", () => {
  expect(
    gated([
      group("a"),
      group("b", {
        gate: "merged",
      }),
    ]),
  ).toBe(true);
  expect(
    gated([
      group("a"),
      group("b"),
    ]),
  ).toBe(false);
  expect(gated([])).toBe(false);
});
