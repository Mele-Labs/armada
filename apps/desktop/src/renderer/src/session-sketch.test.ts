// The sketch put to him that a Session pins is the one drawn last, answered or not: a newer settled sketch pins
// nothing, and an older one still open must not take its place.

import { expect, test } from "vitest";
import type { SessionAttachment } from "@armada/screens/src/draft/sessions";
import { newestAskedSketch } from "./session-sketch";

type Sketch = Extract<
  SessionAttachment,
  {
    kind: "sketch";
  }
>;

function sketch(id: string, more: Partial<Sketch> = {}): Sketch {
  return {
    kind: "sketch",
    id,
    title: id,
    by: "agent",
    drawing: {
      nodes: [],
      edges: [],
    },
    ...more,
  };
}

const older = "ab12cd34-20261104T151426339Z-1";
const newer = "ab12cd34-20261104T163000000Z-1";

test("the newest asked sketch is the one drawn last, whatever the list order", () => {
  const first = sketch("first", {
    asked: true,
    drawn: older,
  });
  const second = sketch("second", {
    asked: true,
    drawn: newer,
  });
  expect(
    newestAskedSketch([
      first,
      second,
    ])?.id,
  ).toBe("second");
  expect(
    newestAskedSketch([
      second,
      first,
    ])?.id,
  ).toBe("second");
});

test("a newer settled sketch beats an older open one, so nothing is pinned", () => {
  const open = sketch("open", {
    asked: true,
    drawn: older,
  });
  const settled = sketch("settled", {
    asked: true,
    drawn: newer,
    settled: {
      how: "approved",
      at: "2026-11-04T16:31:00Z",
    },
  });
  const got = newestAskedSketch([
    open,
    settled,
  ]);
  expect(got?.id).toBe("settled");
  expect(got?.settled).toBeDefined();
});

test("sketches not put to him are ignored, even when drawn later", () => {
  const asked = sketch("asked", {
    asked: true,
    drawn: older,
  });
  const shown = sketch("shown", {
    drawn: newer,
  });
  expect(
    newestAskedSketch([
      asked,
      shown,
    ])?.id,
  ).toBe("asked");
  expect(
    newestAskedSketch([
      shown,
    ]),
  ).toBeUndefined();
  expect(newestAskedSketch([])).toBeUndefined();
});

test("two sketches drawn in the same instant go by the number after it", () => {
  const one = sketch("one", {
    asked: true,
    drawn: "ab12cd34-20261104T151426339Z-2",
  });
  const two = sketch("two", {
    asked: true,
    drawn: "ab12cd34-20261104T151426339Z-10",
  });
  expect(
    newestAskedSketch([
      two,
      one,
    ])?.id,
  ).toBe("two");
  expect(
    newestAskedSketch([
      one,
      two,
    ])?.id,
  ).toBe("two");
});

test("with no drawn row on any, the first asked in list order wins", () => {
  const a = sketch("a", {
    asked: true,
  });
  const b = sketch("b", {
    asked: true,
  });
  expect(
    newestAskedSketch([
      a,
      b,
    ])?.id,
  ).toBe("a");
});
