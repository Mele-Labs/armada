import { expect, it } from "vitest";

import { apply } from "../shared/state-change";
import { StateSends } from "./state-sends";

it("sends a window the whole state first, then only what moved, and nothing where nothing did", () => {
  const sends = new StateSends<{ jobs: { id: string; n: number }[] }>();
  const window = {};
  const one = { jobs: [{ id: "a", n: 1 }, { id: "b", n: 1 }] };
  expect(sends.next(window, one)).toEqual({ full: one, seq: 1 });
  expect(sends.next(window, structuredClone(one))).toBeNull();
  const two = { jobs: [{ id: "a", n: 1 }, { id: "b", n: 2 }] };
  const change = sends.next(window, two)!;
  expect(change).toMatchObject({ from: 1, to: 2 });
  expect("patch" in change && apply(one, change.patch)).toEqual(two);
});

it("keeps each window's own base, and starts a window it asks the whole state of from there", () => {
  const sends = new StateSends<{ n: number }>();
  const a = {};
  const b = {};
  sends.next(a, { n: 1 });
  expect(sends.next(b, { n: 1 })).toEqual({ full: { n: 1 }, seq: 1 });
  expect(sends.full(a, { n: 5 })).toEqual({ full: { n: 5 }, seq: 2 });
  expect(sends.next(a, { n: 6 })).toMatchObject({ from: 2, to: 3 });
});
