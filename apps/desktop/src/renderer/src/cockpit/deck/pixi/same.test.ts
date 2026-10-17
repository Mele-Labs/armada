// Whether two scenes are the same picture. The page hands the canvas a fresh scene each time its clock moves, with every list and map in
// it rebuilt; an unchanged one must compare equal, or an idle map would draw every five seconds for nothing.

import { expect, test } from "vitest";

import { same } from "./same";

test("a scene rebuilt with the same content is the same picture, though none of its parts is the same object", () => {
  const scene = () => ({ heights: new Map([["a", 4], ["b", 0]]), drones: [{ key: "session:a", at: { x: 1, y: 2 }, loop: false }], lit: new Set(["a"]), readable: true });
  expect(same(scene(), scene())).toBe(true);
});

test("a change anywhere in it is a different picture", () => {
  const base = { heights: new Map([["a", 4]]), drones: [{ key: "session:a", at: { x: 1, y: 2 } }], lit: new Set(["a"]) };
  expect(same(base, { ...base, heights: new Map([["a", 5]]) })).toBe(false);
  expect(same(base, { ...base, heights: new Map([["a", 4], ["b", 0]]) })).toBe(false);
  expect(same(base, { ...base, drones: [{ key: "session:a", at: { x: 1, y: 3 } }] })).toBe(false);
  expect(same(base, { ...base, drones: [] })).toBe(false);
  expect(same(base, { ...base, lit: new Set(["b"]) })).toBe(false);
  expect(same(base, { ...base, lit: new Set() })).toBe(false);
});

test("the clock a build carries is not drawn, so it is not compared", () => {
  expect(same({ build: { key: "k", now: 1, tasks: [1] } }, { build: { key: "k", now: 99_000, tasks: [1] } })).toBe(true);
  expect(same({ build: { key: "k", now: 1, tasks: [1] } }, { build: { key: "k", now: 99_000, tasks: [2] } })).toBe(false);
});

test("an array is not an object with the same keys", () => {
  expect(same([1], { 0: 1 })).toBe(false);
});
