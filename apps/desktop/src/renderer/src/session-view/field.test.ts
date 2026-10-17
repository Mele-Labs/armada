// The Field draws only the subagents at work: a finished one is gone from it, and the Session's own drone stays.

import { expect, test } from "vitest";
import type { Session, WorkDrone } from "@armada/screens/src/draft/sessions";

import { workingDrones } from "./field";

const drone = (id: string, agent?: string): WorkDrone => ({ id, ...(agent === undefined ? {} : { agent }), path: "crates/fleet", doing: "write", file: "crates/fleet/src/lib.rs" });
const session = (state: "running" | "done") =>
  ({ id: "s1", attachments: [{ kind: "subagent", id: "sa-1", task: "Read", state }, { kind: "subagent", id: "sa-2", task: "Write", state: "running" }], rows: [], turn: { state: "idle" } }) as unknown as Session;
const FIELD = [drone("main"), drone("sa-1", "Explore"), drone("sa-2", "rust-engineer")];

test("a subagent that has finished leaves the Field, and the ones at work and the Session's own stay", () => {
  expect(workingDrones(session("done"), FIELD)?.map((one) => one.id)).toEqual(["main", "sa-2"]);
  expect(workingDrones(session("running"), FIELD)?.map((one) => one.id)).toEqual(["main", "sa-1", "sa-2"]);
});

test("with no field there is nothing to cut", () => {
  expect(workingDrones(session("done"), undefined)).toBeUndefined();
});
