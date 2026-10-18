// The Field draws only the subagents at work: a finished one is gone from it, and the Session's own drone stays.

import { expect, test } from "vitest";
import type { Session, WorkDrone } from "@armada/screens/src/draft/sessions";

import { isWorking, workingDrones } from "./field";

const drone = (id: string, agent?: string): WorkDrone => ({ id, ...(agent === undefined ? {} : { agent }), path: "crates/fleet", doing: "write", file: "crates/fleet/src/lib.rs" });
const session = (state: "running" | "done") =>
  ({ id: "s1", attachments: [{ kind: "subagent", id: "sa-1", task: "Read", state }, { kind: "subagent", id: "sa-2", task: "Write", state: "running" }], rows: [], turn: { state: "idle" } }) as unknown as Session;
const withAgents = (agents: { id: string; task: string; state: "running" | "done"; agentType?: string; call?: string }[], turn: "idle" | "working" = "idle") =>
  ({ id: "s1", attachments: agents.map((one) => ({ kind: "subagent", ...one })), rows: [], turn: { state: turn } }) as unknown as Session;
const FIELD = [drone("main"), drone("sa-1", "Explore"), drone("sa-2", "rust-engineer")];

test("a subagent that has finished leaves the Field, and the ones at work and the Session's own stay", () => {
  expect(workingDrones(session("done"), FIELD)?.map((one) => one.id)).toEqual(["main", "sa-2"]);
  expect(workingDrones(session("running"), FIELD)?.map((one) => one.id)).toEqual(["main", "sa-1", "sa-2"]);
});

test("with no field and no subagent at work there is nothing to draw", () => {
  const none = { id: "s1", attachments: [{ kind: "subagent", id: "sa-1", task: "Read", state: "done" }], rows: [], turn: { state: "idle" } } as unknown as Session;
  expect(workingDrones(none, undefined)).toBeUndefined();
});

test("subagents at work that no file has placed are drawn at rest beside the Session's own drone, from the ledger's rows", () => {
  const drawn = workingDrones(withAgents([{ id: "sa-1", task: "Harden the restart scripts", state: "running", agentType: "bridge-engineer" }, { id: "sa-2", task: "Failed refresh toast", state: "running" }]), undefined);
  expect(drawn?.map((one) => one.id)).toEqual(["main", "sa-1", "sa-2"]);
  expect(drawn?.[1]).toMatchObject({ agent: "bridge-engineer", task: "Harden the restart scripts", path: "" });
  expect(drawn?.[2]).toMatchObject({ agent: "Subagent", path: "" });
  expect(drawn?.map((one) => isWorking(session("running"), one))).toEqual([false, true, true]);
});

test("a subagent a file has placed is drawn once, where it is, and one that is done is not added", () => {
  const drawn = workingDrones(withAgents([{ id: "sa-1", task: "Read", state: "running" }, { id: "sa-2", task: "Write", state: "running" }, { id: "sa-3", task: "Old", state: "done" }]), [drone("main"), drone("sa-1", "Explore")]);
  expect(drawn?.map((one) => one.id)).toEqual(["main", "sa-1", "sa-2"]);
  expect(drawn?.[1]).toMatchObject({ path: "crates/fleet", file: "crates/fleet/src/lib.rs" });
});

// Fleet keys a touch by the call that started the subagent, and the ledger's row by the id the engine gave it; the row names the call.
test("a finished subagent leaves the Field when its drone is keyed by the call and the ledger's row by the agent id", () => {
  const ledger = withAgents([
    { id: "a4c6", call: "toolu_1", task: "Needs-you eyebrow mock walk", state: "done", agentType: "bridge-engineer" },
    { id: "a9e7", call: "toolu_2", task: "Fix mock tests", state: "running", agentType: "bridge-engineer" },
  ]);
  const drawn = workingDrones(ledger, [drone("main"), drone("toolu_1", "subagent"), drone("toolu_2", "subagent")]);
  expect(drawn?.map((one) => one.id)).toEqual(["main", "a9e7"]);
});

test("a drone placed by a call is titled by its agent type and its task, and is not drawn a second time from the ledger", () => {
  const ledger = withAgents([{ id: "a9e7", call: "toolu_2", task: "Fix mock tests", state: "running", agentType: "rust-engineer" }]);
  const drawn = workingDrones(ledger, [drone("main"), drone("toolu_2", "subagent")]);
  expect(drawn).toHaveLength(2);
  expect(drawn?.[1]).toMatchObject({ agent: "rust-engineer", task: "Fix mock tests", path: "crates/fleet" });
});

test("a drone is at work only while its Session's turn or its subagent's row is", () => {
  const ledger = (turn: "idle" | "working") => withAgents([{ id: "a1", task: "Read", state: "running" }], turn);
  const field = [drone("main"), drone("a1", "Explore")];
  expect(workingDrones(ledger("idle"), field)?.map((one) => isWorking(ledger("idle"), one))).toEqual([false, true]);
  expect(workingDrones(ledger("working"), field)?.map((one) => isWorking(ledger("working"), one))).toEqual([true, true]);
  const unlisted = withAgents([], "idle");
  expect(isWorking(unlisted, drone("x", "Explore"))).toBe(false);
});
