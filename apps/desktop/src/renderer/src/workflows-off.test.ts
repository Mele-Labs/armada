import { expect, test } from "vitest";

import { isOff, switched, workflowsOffOf } from "./workflows-off";

const NOTHING = { machine: [], repositories: {} };

test("a value that is absent, null, a list, text or a number reads as nothing off", () => {
  for (const bad of [undefined, null, [], ["bug"], "bug", 3, true]) expect(workflowsOffOf(bad)).toEqual(NOTHING);
});

test("the machine's list and each repository's list are read as they were saved", () => {
  const off = workflowsOffOf({ machine: ["prototype"], repositories: { armada: ["bug", "docs_only"], other: ["feature"] } });
  expect(off.machine).toEqual(["prototype"]);
  expect(off.repositories).toEqual({ armada: ["bug", "docs_only"], other: ["feature"] });
});

test("a list that is not a list, or holds more than ids, keeps only the ids", () => {
  expect(workflowsOffOf({ machine: "prototype", repositories: { armada: ["bug", 4, null] } })).toEqual({ machine: [], repositories: { armada: ["bug"] } });
  expect(workflowsOffOf({ machine: ["bug", 2], repositories: "armada" })).toEqual({ machine: ["bug"], repositories: {} });
});

test("a workflow is off for a repository when the machine or that repository has it off, and not for another", () => {
  const off = workflowsOffOf({ machine: ["prototype"], repositories: { armada: ["bug"] } });
  expect(isOff(off, "prototype", "armada")).toBe(true);
  expect(isOff(off, "prototype", undefined)).toBe(true);
  expect(isOff(off, "bug", "armada")).toBe(true);
  expect(isOff(off, "bug", "other")).toBe(false);
  expect(isOff(off, "bug", undefined)).toBe(false);
  expect(isOff(off, "feature", "armada")).toBe(false);
});

test("switching a workflow off with no repository adds it to the machine's list once, and leaves the repositories alone", () => {
  const off = workflowsOffOf({ machine: ["prototype"], repositories: { armada: ["bug"] } });
  expect(switched(off, "docs_only", false, null)).toEqual({ machine: ["prototype", "docs_only"], repositories: { armada: ["bug"] } });
  expect(switched(off, "prototype", false, null)).toEqual({ machine: ["prototype"], repositories: { armada: ["bug"] } });
});

test("switching one back on takes it off the machine's list, and off the repository's list with a repository named", () => {
  const off = workflowsOffOf({ machine: ["prototype", "bug"], repositories: { armada: ["bug", "docs_only"] } });
  expect(switched(off, "prototype", true, null)).toEqual({ machine: ["bug"], repositories: { armada: ["bug", "docs_only"] } });
  expect(switched(off, "bug", true, "armada")).toEqual({ machine: ["prototype", "bug"], repositories: { armada: ["docs_only"] } });
});

test("switching a workflow off in one repository adds it there and in no other", () => {
  const off = workflowsOffOf({ machine: [], repositories: { armada: ["bug"] } });
  expect(switched(off, "docs_only", false, "armada")).toEqual({ machine: [], repositories: { armada: ["bug", "docs_only"] } });
  expect(switched(off, "docs_only", false, "other")).toEqual({ machine: [], repositories: { armada: ["bug"], other: ["docs_only"] } });
});

test("a repository whose last workflow is switched back on has no entry left, and the value read back is nothing off", () => {
  const off = workflowsOffOf({ machine: [], repositories: { armada: ["bug"] } });
  const back = switched(off, "bug", true, "armada");
  expect(back).toEqual({ machine: [], repositories: {} });
  expect(workflowsOffOf(back)).toEqual(NOTHING);
});

test("switching on a workflow that was never off changes nothing", () => {
  const off = workflowsOffOf({ machine: ["prototype"], repositories: { armada: ["bug"] } });
  expect(switched(off, "feature", true, null)).toEqual({ machine: ["prototype"], repositories: { armada: ["bug"] } });
  expect(switched(off, "feature", true, "armada")).toEqual({ machine: ["prototype"], repositories: { armada: ["bug"] } });
  expect(switched(off, "feature", true, "other")).toEqual({ machine: ["prototype"], repositories: { armada: ["bug"] } });
});
