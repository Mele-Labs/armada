// The panel names its Session from the query main loaded it with.

import { expect, it } from "vitest";

import { subjectOf } from "./subject";

it("a Session's panel holds that Session", () => {
  expect(subjectOf("?session=s1")).toEqual({ kind: "session", id: "s1" });
});

it("a query naming none holds nothing", () => {
  expect(subjectOf("?job=01JOB")).toBeNull();
  expect(subjectOf("")).toBeNull();
  expect(subjectOf("?session=")).toBeNull();
});
