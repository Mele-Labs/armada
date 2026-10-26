// Where the story stands, and what it lands in, read off what the Session holds.

import { expect, test } from "vitest";
import type { Session, SessionWork, WorkPlan } from "@armada/screens/src/draft/sessions";

import { landingOf, stageOf } from "./story";

const plan = (approved: boolean, state: "open" | "done"): WorkPlan => ({ scope: [], approved, groups: [{ id: "g1", agent: "rust-engineer", title: "Group", paths: [], tasks: [{ id: "t1", text: "Task", state }] }] });
const session = (work: SessionWork, mod?: true, working?: true): Session => ({ id: "s1", attachments: [], rows: [], turn: { state: working === true ? "working" : "idle" }, work, ...(mod === undefined ? {} : { mod }) }) as unknown as Session;
const at = (phase: "setup" | "work" | "delivery"): SessionWork["workflow"] => ({ name: "bug", steps: ["Plan", "Build", "Hand in"], at: 2, phase });

test("a step in the delivery phase is the landing, though tasks are left", () => {
  expect(stageOf(session({ plan: plan(true, "open"), workflow: at("delivery") }))).toBe("landing");
  expect(stageOf(session({ workflow: at("delivery") }))).toBe("landing");
});

test("a step in the work phase leaves the stage to the plan", () => {
  expect(stageOf(session({ plan: plan(true, "open"), workflow: at("work") }))).toBe("field");
  expect(stageOf(session({ plan: plan(true, "done"), workflow: at("work") }))).toBe("landing");
  expect(stageOf(session({ workflow: at("setup") }))).toBe("blueprint");
});

test("a Job with no plan is the field while its turn runs, and the blueprint while it waits", () => {
  expect(stageOf(session({}, undefined, true))).toBe("field");
  expect(stageOf(session({}))).toBe("blueprint");
});

test("a plan not approved is the blueprint, whatever step the workflow is at", () => {
  expect(stageOf(session({ plan: plan(false, "open"), workflow: at("delivery") }))).toBe("blueprint");
});

test("the landing is the workflow's, as Fleet says it", () => {
  expect(landingOf(session({ landing: "spike" }))).toBe("spike");
  expect(landingOf(session({ landing: "report" }))).toBe("report");
  expect(landingOf(session({ landing: "document" }))).toBe("document");
  expect(landingOf(session({}))).toBe("pull_request");
});

test("a Mod Job lands its mod, whatever its workflow lands in", () => {
  expect(landingOf(session({ landing: "pull_request" }, true))).toBe("mod");
  expect(landingOf(session({}, true))).toBe("mod");
});
