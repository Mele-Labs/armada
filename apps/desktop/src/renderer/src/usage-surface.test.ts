// The titles the Usage page joins Jobs to are the ones Bridge already holds.

import { expect, test } from "vitest";

import { titlesOf } from "./usage-surface";

test("titles are read from the sessions held, and a session with none is left out", () => {
  const record = (id: string, title?: string) => ({ id, ...(title === undefined ? {} : { title }) }) as never;
  const titles = titlesOf({ state: "read", sessions: [record("a", "Cockpit"), record("b"), record("c", "")] });
  expect([...titles]).toEqual([["a", "Cockpit"]]);
});

test("before Fleet has answered there are no titles", () => {
  expect(titlesOf({ state: "none" }).size).toBe(0);
});
