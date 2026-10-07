import { expect, test } from "vitest";
import { CAP, fold, start, type Place } from "./history";

const at = (surface: string, job: string | null = null): Place => ({ surface, job, session: null, studio: null, studioNode: null });
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
