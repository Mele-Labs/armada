import { expect, test } from "vitest";
import { CAP, collapse, fold, openingOf, start, type Place } from "./history";

const at = (surface: string, job: string | null = null, tab: Place["tab"] = null, item: string | null = null): Place => ({ surface, job, tab, item, session: null, studio: null, studioNode: null });
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

test("a Job's first tab completes its visit instead of making a second", () => {
  let h = visit(start(at("a")), at("b", "j"));
  h = visit(h, at("b", "j", "overview"));
  expect(h.back).toEqual([at("a")]);
  expect(h.at).toEqual(at("b", "j", "overview"));
});

test("a change of tab on an open Job is a visit", () => {
  const h = visit(visit(visit(start(at("a")), at("b", "j")), at("b", "j", "overview")), at("b", "j", "plan"));
  expect(h.back).toEqual([at("a"), at("b", "j", "overview")]);
});

test("the blank render before a restored Job's tab reports is not a visit", () => {
  const h = start(at("b", "j", "plan"));
  expect(visit(h, at("b", "j"))).toBe(h);
});

test("a Job reported straight onto a tab and item is still one visit", () => {
  let h = visit(start(at("a")), at("b", "j"));
  h = visit(h, at("b", "j", "plan", "T2"));
  expect(h.back).toEqual([at("a")]);
  expect(h.at).toEqual(at("b", "j", "plan", "T2"));
});

test("selecting inside a tab is a visit, and so is closing it", () => {
  let h = visit(start(at("b", "j", "plan")), at("b", "j", "plan", "T1"));
  h = visit(h, at("b", "j", "plan", "T2"));
  h = visit(h, at("b", "j", "plan"));
  expect(h.back).toEqual([at("b", "j", "plan"), at("b", "j", "plan", "T1"), at("b", "j", "plan", "T2")]);
});

test("switching tab from a selection is one visit, not two", () => {
  const h = visit(start(at("b", "j", "plan", "T2")), at("b", "j", "drones"));
  expect(h.back).toEqual([at("b", "j", "plan", "T2")]);
  expect(h.at).toEqual(at("b", "j", "drones"));
});

test("the blank render before a restored selection's tab reports is not a visit", () => {
  const h = start(at("b", "j", "plan", "T2"));
  expect(visit(h, at("b", "j"))).toBe(h);
});

test("back steps through selections and forward returns", () => {
  let h = visit(start(at("b", "j", "plan")), at("b", "j", "plan", "T1"));
  h = visit(h, at("b", "j", "plan", "T2"));
  h = visit(h, at("b", "j", "drones"));
  const back = fold(fold(h, { kind: "back" }), { kind: "back" });
  expect(back.at).toEqual(at("b", "j", "plan", "T1"));
  expect(fold(back, { kind: "forward" }).at).toEqual(at("b", "j", "plan", "T2"));
});

test("an item opens by the kind its tab holds", () => {
  expect(openingOf(at("b", "j", "plan", "T2"))).toEqual({ tab: "plan", task: "T2" });
  expect(openingOf(at("b", "j", "workflow", "s"))).toEqual({ tab: "workflow", step: "s" });
  expect(openingOf(at("b", "j", "drones", "d"))).toEqual({ tab: "drones", drone: "d" });
  expect(openingOf(at("b", "j", "record", "7"))).toEqual({ tab: "record", row: "7" });
  expect(openingOf(at("b", "j", "plan"))).toEqual({ tab: "plan" });
  expect(openingOf(at("b", "j"))).toEqual({});
});

// The flight deck opens pages as layers over the Cockpit. A press on the Cockpit tab, or on a sibling page's, replaces
// the layers rather than stacking on them, so Back lands on the Cockpit and not on the last layer.
const cockpit = at("overview");

test("Back from a sibling returns to the Cockpit", () => {
  const layered = visit(visit(start(cockpit), at("jobs")), at("jobs", "j"));
  const sibling = collapse(layered, at("sessions"));
  expect(sibling).toEqual({ back: [cockpit], at: at("sessions"), forward: [] });
  expect(fold(sibling, { kind: "back" }).at).toEqual(cockpit);
});

test("a press on the Cockpit drops the layers over it and keeps what was behind it", () => {
  let h = visit(start(at("x")), cockpit);
  h = visit(visit(h, at("jobs")), at("jobs", "j"));
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
