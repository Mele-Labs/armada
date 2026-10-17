import { expect, test } from "vitest";
import { CAP, collapse, fold, start, type Place } from "./history";

const at = (surface: string): Place => ({ surface, studio: null, studioNode: null });
const visit = (h: ReturnType<typeof start>, place: Place) => fold(h, { kind: "visited", place });

test("a visit pushes where you were", () => {
  const h = visit(start(at("a")), at("b"));
  expect(h).toEqual({ back: [at("a")], at: at("b"), forward: [] });
});

test("the same place twice is one visit", () => {
  const h = visit(start(at("a")), at("a"));
  expect(h.back).toEqual([]);
});

test("back then forward returns", () => {
  const h = visit(visit(start(at("a")), at("b")), at("c"));
  const back = fold(h, { kind: "back" });
  expect(back.at).toEqual(at("b"));
  expect(back.forward).toEqual([at("c")]);
  expect(fold(back, { kind: "forward" })).toEqual(h);
});

test("a new visit after back drops forward", () => {
  const h = fold(visit(start(at("a")), at("b")), { kind: "back" });
  expect(visit(h, at("c")).forward).toEqual([]);
});

test("back and forward at the ends do nothing", () => {
  const h = start(at("a"));
  expect(fold(h, { kind: "back" })).toBe(h);
  expect(fold(h, { kind: "forward" })).toBe(h);
});

test("the stack is capped, oldest dropped", () => {
  let h = start(at("0"));
  for (let i = 1; i <= CAP + 10; i++) h = visit(h, at(String(i)));
  expect(h.back).toHaveLength(CAP);
  expect(h.back[0]).toEqual(at("10"));
});

// The flight deck opens pages as layers over the Cockpit. A press on the Cockpit tab, or on a sibling page's, replaces
// the layers rather than stacking on them, so Back lands on the Cockpit and not on the last layer.
const cockpit = at("overview");

test("Back from a sibling returns to the Cockpit", () => {
  const layered = visit(visit(start(cockpit), at("checks")), at("lessons"));
  const sibling = collapse(layered, at("sessions"));
  expect(sibling).toEqual({ back: [cockpit], at: at("sessions"), forward: [] });
  expect(fold(sibling, { kind: "back" }).at).toEqual(cockpit);
});

test("a press on the Cockpit drops the layers over it and keeps what was behind it", () => {
  let h = visit(start(at("x")), cockpit);
  h = visit(visit(h, at("checks")), at("lessons"));
  const forward = fold(h, { kind: "back" });
  expect(forward.forward).toHaveLength(1);
  expect(collapse(forward, cockpit)).toEqual({ back: [at("x")], at: cockpit, forward: [] });
});

test("a press made on the Cockpit itself keeps it behind the sibling", () => {
  const h = visit(start(at("x")), cockpit);
  expect(collapse(h, at("sessions"))).toEqual({ back: [at("x"), cockpit], at: at("sessions"), forward: [] });
});

test("with no Cockpit visit anywhere a collapse is a plain visit that forgets forward", () => {
  const h = fold(visit(visit(start(at("a")), at("b")), at("c")), { kind: "back" });
  expect(collapse(h, at("d"))).toEqual({ back: [at("a"), at("b")], at: at("d"), forward: [] });
});
