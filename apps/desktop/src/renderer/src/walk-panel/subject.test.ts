// The panel names its kind from the query main loaded it with, so a Session's walk is never drawn as a Job's.

import { expect, it } from "vitest";

import { subjectOf } from "./subject";

it("a Session's panel holds that Session", () => {
  expect(subjectOf("?session=s1")).toEqual({ kind: "session", id: "s1" });
});

it("a Job's panel holds that Job", () => {
  expect(subjectOf("?job=01JOB")).toEqual({ kind: "job", id: "01JOB" });
});

it("a query naming both, neither, or nothing holds nothing", () => {
  expect(subjectOf("?session=s1&job=01JOB")).toBeNull();
  expect(subjectOf("")).toBeNull();
  expect(subjectOf("?session=")).toBeNull();
});
