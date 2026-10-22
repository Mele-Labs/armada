import { describe, expect, it } from "vitest";

import type { BuildPiece, FileTouch, SessionPlan, SessionRecord, SessionRow as WireRow, TimelineEvent, WaitingItem, WorkflowSummary } from "@armada/protocol";
import type { PlanMark, SessionAttachment } from "./draft/sessions";
import { sessionOfRecord } from "./sessions-wire";
import type { Beside } from "./sessions-wire";
import { changeOfEvent, choicesOf, concernsOf, fieldOf, linesOfWire, planOf, tasksOf, workOf, wireMarksOf } from "./session-work";

const AT = "2026-10-16T09:00:00Z";

const record = (change: Partial<SessionRecord> = {}): SessionRecord => ({
  id: "s1",
  harness: "a_harness",
  origin: "bridge",
  manifest_id: "armada",
  cwd: "/repo",
  state: "live",
  started_at: AT,
  last_seen_at: AT,
  usage: {},
  attachments: [],
  hosted: { turn: { state: "idle" }, mode: "auto", running: false },
  ...change,
});

const step = (step_id: string, label: string) => ({ step_id, label, checks: [], judge_checks: [] }) as unknown as WorkflowSummary["steps"][number];
const FEATURE = { id: "feature", name: "Feature", version: 1, manifest_id: "armada", steps: [step("plan", "Plan"), step("build", "Build"), step("pr", "Pull request")] } as WorkflowSummary;

const beside = (more: Partial<Parameters<typeof workOf>[1]> = {}): Parameters<typeof workOf>[1] => ({ workflows: [FEATURE], attachments: [], rows: undefined, drawn: [], unpicked: new Set(), ...more });

const PLAN: SessionPlan = {
  version: 2,
  summary: "",
  recorded_at: AT,
  approved: false,
  scope: ["crates/fleet"],
  groups: [{ id: "g1", agent: "rust-engineer", title: "Retry the lease", paths: ["crates/fleet"], tasks: [{ id: "t1", text: "Back off", state: "working", seen: true }, { id: "t2", text: "Cap it", state: "open" }] }],
  changes: { added: ["t2"], renamed: [{ id: "t1", from: "Wait" }], moved: [], removed: [{ group: "g1", text: "Sleep" }] },
  sketch: { id: "sketch:Plan", title: "Plan" },
};

describe("the proposer's picks", () => {
  it("are listed for a hosted Session, and the ones he changed are not", () => {
    const one = record({ workflow: "feature", proposal: { picked: ["title", "workflow", "model", "effort"] } });
    expect(workOf(one, beside())?.proposal).toEqual({ workflow: "feature", picked: ["workflow", "model", "effort"] });
    expect(workOf(one, beside({ unpicked: new Set(["model", "workflow"]) }))?.proposal).toEqual({ workflow: "feature", picked: ["effort"] });
    expect(workOf(one, beside({ unpicked: new Set(["model", "workflow", "effort"]) }))?.proposal).toEqual({ workflow: "feature" });
  });

  it("carry the proposer reading, and a custom Session has no workflow", () => {
    expect(workOf(record({ proposal: { reading: true } }), beside())?.proposal).toEqual({ workflow: null, reading: true });
  });

  it("are not a terminal Session's, which Fleet does not set a workflow for", () => {
    const terminal = record({ hosted: undefined, terminal: { listening: true } as SessionRecord["terminal"] });
    expect(workOf(terminal, beside())?.proposal).toBeUndefined();
  });

  it("reach the Session the view draws", () => {
    const session = sessionOfRecord(record({ workflow: "feature", proposal: { picked: ["effort"] } }), undefined, { picture: () => undefined, sketches: [], pending: [], workflows: [FEATURE] } as Beside);
    expect(session.work?.proposal?.picked).toEqual(["effort"]);
    const changed = sessionOfRecord(record({ proposal: { picked: ["effort"] } }), undefined, { picture: () => undefined, sketches: [], pending: [], unpicked: new Set(["effort"]) } as Beside);
    expect(changed.work?.proposal?.picked).toBeUndefined();
  });
});

describe("the workflow", () => {
  it("draws the steps unlit while Fleet names none", () => {
    expect(workOf(record({ workflow: "feature" }), beside())?.workflow).toEqual({ name: "Feature", steps: ["Plan", "Build", "Pull request"] });
  });

  it("lights the step Fleet names", () => {
    expect(workOf(record({ workflow: "feature", workflow_step: 1 }), beside())?.workflow).toEqual({ name: "Feature", steps: ["Plan", "Build", "Pull request"], at: 1 });
    expect(workOf(record({ workflow: "feature", workflow_step: 0 }), beside())?.workflow?.at).toBe(0);
    expect(workOf(record({ workflow: "feature", workflow_step: 9 }), beside())?.workflow?.at).toBeUndefined();
  });

  it("is hidden while the setting turns workflows off: no steps, no picker, no mark for a workflow", () => {
    const one = record({ workflow: "feature", workflow_step: 1, proposal: { picked: ["workflow", "model"] } });
    const work = workOf(one, beside({ workflowsOff: true }));
    expect(work?.railOff).toBe(true);
    expect(work?.workflow).toBeUndefined();
    expect(work?.proposal?.picked).toEqual(["model"]);
    expect(workOf(one, beside())?.railOff).toBeUndefined();
  });

  it("has no steps for a custom Session, and lists the picker's choices by id", () => {
    expect(workOf(record(), beside())?.workflow).toBeUndefined();
    expect(choicesOf([FEATURE])).toEqual([{ id: "feature", name: "Feature", steps: ["Plan", "Build", "Pull request"], manifestId: "armada" }]);
  });

  it("is a Helm Session's none", () => {
    expect(workOf(record({ helm: true }), beside())).toBeUndefined();
  });
});

describe("the plan", () => {
  const sketch: SessionAttachment = { kind: "sketch", id: "sketch:Plan", title: "Plan", by: "agent", drawing: { nodes: [], edges: [] } };

  it("keeps Fleet's task states and what it changed, with the sketch the ledger holds", () => {
    const plan = planOf(PLAN, { attachments: [sketch], rows: undefined, drawn: [] });
    expect(plan.version).toBe(2);
    expect(plan.groups[0]?.tasks).toEqual([{ id: "t1", text: "Back off", state: "working", seen: true }, { id: "t2", text: "Cap it", state: "open" }]);
    expect(plan.changes).toEqual({ added: ["t2"], renamed: [{ id: "t1", from: "Wait" }], removed: [{ group: "g1", text: "Sleep" }] });
    expect(plan.sketch).toEqual({ id: "sketch:Plan", title: "Plan", scene: { nodes: [], edges: [] } });
  });

  it("keeps the issue a group names, and names none where it names none", () => {
    const named: SessionPlan = { ...PLAN, groups: [{ ...PLAN.groups[0]!, issue: "2214" }, { id: "g2", agent: "bridge-engineer", title: "Draw it", paths: [], tasks: [] }] };
    const groups = planOf(named, { attachments: [], rows: undefined, drawn: [] }).groups;
    expect(groups[0]?.issue).toBe("2214");
    expect(groups[1]).not.toHaveProperty("issue");
  });

  it("leaves the sketch off until the ledger holds it", () => {
    expect(planOf(PLAN, { attachments: [], rows: undefined, drawn: [] }).sketch).toBeUndefined();
  });

  it("settles after the last row before the one that recorded it", () => {
    const rows = [
      { kind: "message", id: "m1", at: AT, from: { kind: "you" }, text: "go" },
      { kind: "message", id: "m2", at: AT, from: { kind: "agent" }, text: "planned" },
      { kind: "plan", id: "p1", at: AT, version: 2 },
    ] as WireRow[];
    const drawn = [{ id: "m1" }, { id: "m2" }] as never;
    expect(planOf(PLAN, { attachments: [], rows, drawn }).after).toBe("m2");
    expect(planOf({ ...PLAN, version: 3 }, { attachments: [], rows, drawn }).after).toBeUndefined();
  });
});

describe("the Field", () => {
  const touch = (path: string, act: FileTouch["act"], at: string, by?: FileTouch["by"]): FileTouch => ({ path, act, at, ...(by === undefined ? {} : { by }) });

  it("puts the Session's drone and one for each subagent at the last file each touched", () => {
    const { field, files } = fieldOf([
      touch("crates/fleet/src/lease.rs", "edit", "2026-10-16T09:00:01Z"),
      touch("crates/ipc/src/lease.rs", "write", "2026-10-16T09:00:02Z", { agent: "protocol-engineer", id: "a1" }),
      touch("crates/fleet/src/retry.rs", "write", "2026-10-16T09:00:03Z"),
    ]);
    expect(field).toEqual([
      { id: "main", path: "crates/fleet", doing: "write", file: "crates/fleet/src/retry.rs", at: "2026-10-16T09:00:03Z" },
      { id: "a1", agent: "protocol-engineer", path: "crates/ipc", doing: "write", file: "crates/ipc/src/lease.rs", at: "2026-10-16T09:00:02Z" },
    ]);
    expect(files).toEqual([
      { path: "crates/fleet/src/lease.rs", act: "edit" },
      { path: "crates/ipc/src/lease.rs", act: "write" },
      { path: "crates/fleet/src/retry.rs", act: "write" },
    ]);
  });

  it("marks a file with what was done to it last, a removal included", () => {
    const { files } = fieldOf([touch("a/b/c.rs", "write", "2026-10-16T09:00:01Z"), touch("a/b/c.rs", "remove", "2026-10-16T09:00:02Z")]);
    expect(files).toEqual([{ path: "a/b/c.rs", act: "remove" }]);
  });

  it("is empty with no touches", () => {
    expect(fieldOf([])).toEqual({ field: [], files: [] });
    expect(workOf(record(), beside())?.field).toBeUndefined();
  });
});

describe("a judge's concerns", () => {
  const item = (id: string, source: WaitingItem["source"], more: Partial<WaitingItem> = {}): WaitingItem => ({ id, text: id, since: AT, source, ...more });

  it("are the waiting items with source judge, citing a file and a line", () => {
    const items = [item("a", "agent"), item("c1", "judge", { text: "No ceiling", cites: [{ path: "crates/fleet/src/backoff.rs", line: 12 }, { path: "crates/fleet/src/lease.rs" }] }), item("plan:1", "plan")];
    expect(concernsOf(items)).toEqual([{ id: "c1", text: "No ceiling", cites: ["crates/fleet/src/backoff.rs:12", "crates/fleet/src/lease.rs"] }]);
  });

  it("are none once the Session has ended", () => {
    expect(workOf(record({ state: "ended", waiting_for: [item("c1", "judge")] }), beside())?.concerns).toBeUndefined();
  });

  it("name the step they were raised after, and the tile's carry none", () => {
    const from = (concern: WaitingItem["concern"]) => concernsOf([item("c1", "judge", { concern })])[0];
    expect(from({ from: { kind: "step", index: 1, label: "Build" } })?.step).toBe("Build");
    expect(from({ from: { kind: "tile" } })?.step).toBeUndefined();
  });

  it("carry the refusal's parts where the record has them, and none where it was kept as text alone", () => {
    const parts = { from: { kind: "tile" } as const, expected: "A diff touching dispatch-panel", produced: "No file has changed yet", consequence: "Nothing reflects the feature" };
    expect(concernsOf([item("c1", "judge", { concern: parts })])[0]).toMatchObject({ expected: "A diff touching dispatch-panel", produced: "No file has changed yet", consequence: "Nothing reflects the feature" });
    const old = concernsOf([item("c2", "judge", { concern: { from: { kind: "tile" } } })])[0]!;
    expect(["expected", "produced", "consequence"].filter((key) => key in old)).toEqual([]);
  });

  it("show the tile working while Fleet says a judge is reading", () => {
    expect(workOf(record({ judging: true }), beside())?.judging).toBe(true);
    expect(workOf(record(), beside())?.judging).toBeUndefined();
    expect(workOf(record({ state: "ended", judging: true }), beside())?.judging).toBeUndefined();
  });

  it("carry the last reading, and zero concerns is the reading the tile calls No concerns", () => {
    expect(workOf(record({ judge: { at: "2026-10-16T09:05:00Z", concerns: 0 } }), beside())?.judged).toEqual({ at: "2026-10-16T09:05:00Z", concerns: 0 });
    expect(workOf(record(), beside())?.judged).toBeUndefined();
  });
});

describe("the concerns he settled", () => {
  const settledItem = (id: string, how: "sent" | "dismissed", from: NonNullable<WaitingItem["concern"]>["from"] = { kind: "tile" }): WaitingItem => ({
    id,
    text: `${id} text`,
    since: AT,
    source: "judge",
    concern: { from, settled: { how, at: "2026-10-16T09:03:30Z" } },
  });
  const rows = [
    { kind: "message", id: "m1", at: "2026-10-16T09:00:00Z", from: { kind: "you" }, text: "go" },
    { kind: "message", id: "m2", at: "2026-10-16T09:04:00Z", from: { kind: "agent" }, text: "ok" },
  ] as WireRow[];
  const around = { picture: () => undefined, sketches: [], pending: [] } as Beside;

  it("draw a dismissed one as the folded line, at the step it came from", () => {
    const session = sessionOfRecord(record({ settled: [settledItem("j1", "dismissed", { kind: "step", index: 1, label: "Build" })] }), rows, around);
    expect(session.rows.map((one) => one.kind)).toEqual(["message", "concern", "message"]);
    expect(session.rows[1]).toMatchObject({ id: "settled:j1", step: "Build", text: "j1 text" });
  });

  it("draw a sent one as his message, in the thread where it was settled", () => {
    const session = sessionOfRecord(record({ settled: [settledItem("j2", "sent")] }), rows, around);
    expect(session.rows.map((one) => one.kind)).toEqual(["message", "message", "message"]);
    expect(session.rows[1]).toMatchObject({ id: "settled:j2", from: { kind: "you" }, text: "j2 text" });
  });

  it("draw a sent one once, where the thread already carries his message", () => {
    const carried = [...rows, { kind: "message", id: "m3", at: "2026-10-16T09:03:31Z", from: { kind: "you" }, text: "From the judge: j2 text" }] as WireRow[];
    const session = sessionOfRecord(record({ settled: [settledItem("j2", "sent")] }), carried, around);
    expect(session.rows.filter((one) => one.kind === "message" && one.text.includes("j2 text"))).toHaveLength(1);
  });

  it("are not waiting concerns", () => {
    expect(workOf(record({ settled: [settledItem("j1", "sent")] }), beside())?.concerns).toBeUndefined();
  });
});

describe("what he marked on the plan, as Fleet takes it", () => {
  it("renames the added task and group, the move and the subagent, and keeps the rest", () => {
    const marks: PlanMark[] = [
      { kind: "rename", group: "g1", task: "t1", from: "Back off", to: "Back off with a ceiling" },
      { kind: "remove", group: "g1" },
      { kind: "add", group: "g1", task: "t-new-1", id: "t-new-1", text: "Log it" },
      { kind: "add", group: "g-new-1", id: "g-new-1", text: "Docs" },
      { kind: "move", task: "t2", from: "g1", to: "g2" },
      { kind: "agent", group: "g1", from: "rust-engineer", to: "general-purpose" },
      { kind: "note", id: "n1", group: "g1", text: "Reuse a field?" },
    ];
    expect(wireMarksOf(marks)).toEqual([
      { kind: "rename", group: "g1", task: "t1", from: "Back off", to: "Back off with a ceiling" },
      { kind: "remove", group: "g1" },
      { kind: "add_task", group: "g1", id: "t-new-1", text: "Log it" },
      { kind: "add_group", id: "g-new-1", text: "Docs" },
      { kind: "move_task", task: "t2", from: "g1", to: "g2" },
      { kind: "subagent", group: "g1", from: "rust-engineer", to: "general-purpose" },
      { kind: "note", id: "n1", group: "g1", text: "Reuse a field?" },
    ]);
  });
});

describe("the thread's rows", () => {
  it("draws the changes he sent as one row with a line for each", () => {
    expect(linesOfWire([{ verb: "Renamed", place: "Retry", from: "A", text: "B" }, { verb: "Odd", place: "Retry" }])).toEqual([
      { verb: "Renamed", where: "Retry", from: "A", text: "B" },
      { verb: "Note", where: "Retry" },
    ]);
  });

  it("reads a value he changed off the timeline entry", () => {
    expect(changeOfEvent("session.workflow", "workflow: Feature → custom")).toEqual({ what: "Workflow", from: "Feature", to: "custom" });
    expect(changeOfEvent("session.model", "model: opus → sonnet")).toEqual({ what: "Model", from: "opus", to: "sonnet" });
    expect(changeOfEvent("session.effort", "effort: high → medium")).toEqual({ what: "Effort", from: "high", to: "medium" });
    expect(changeOfEvent("session.started", "Started")).toBeUndefined();
  });

  it("puts the plan changes and the value changes in the thread where they happened", () => {
    const rows = [
      { kind: "message", id: "m1", at: "2026-10-16T09:00:00Z", from: { kind: "you" }, text: "go" },
      { kind: "plan_changes", id: "pc", at: "2026-10-16T09:02:00Z", version: 1, lines: [{ verb: "Removed", place: "Retry" }] },
      { kind: "message", id: "m2", at: "2026-10-16T09:03:00Z", from: { kind: "agent" }, text: "ok" },
    ] as WireRow[];
    const events = [
      { id: 7, at: "2026-10-16T09:01:00Z", kind: "session.workflow", subject: { kind: "session", id: "s1" }, tone: "idle", says: "workflow: Feature → custom" },
      { id: 8, at: "2026-10-16T09:01:30Z", kind: "session.model", subject: { kind: "session", id: "other" }, tone: "idle", says: "model: a → b" },
    ] as TimelineEvent[];
    const session = sessionOfRecord(record(), rows, { picture: () => undefined, sketches: [], pending: [], events });
    expect(session.rows.map((one) => one.kind)).toEqual(["message", "change", "suggest", "message"]);
    expect(session.rows[1]).toMatchObject({ id: "event:7", what: "Workflow", from: "Feature", to: "custom" });
    expect(session.rows[2]).toMatchObject({ lines: [{ verb: "Removed", where: "Retry" }] });
  });
});

describe("the agent's own tasks", () => {
  const piece = (id: string, text: string, kind: BuildPiece["piece"], working?: true): BuildPiece => ({ id, text, piece: kind, ...(working === undefined ? {} : { working }) });
  const build = (pieces: BuildPiece[]): SessionRecord["build"] => ({ phase: "implement", pieces, stops: [], forecast: [], stalled: false, workers: [] });
  const PIECES = [piece("1", "Read the runner", "solid"), piece("2", "List the files", "hollow"), piece("3", "Rename them", "scaffold", true), piece("4", "Update the tests", "ghost"), piece("5", "Run the checks", "ghost")];

  it("are the build record's pieces in the order made, a finished piece done, one in progress working and the rest open", () => {
    expect(tasksOf(PIECES)).toEqual([
      { id: "1", subject: "Read the runner", state: "done" },
      { id: "2", subject: "List the files", state: "done" },
      { id: "3", subject: "Rename them", state: "working" },
      { id: "4", subject: "Update the tests", state: "open" },
      { id: "5", subject: "Run the checks", state: "open" },
    ]);
  });

  it("are the work of a Session with no plan", () => {
    expect(workOf(record({ build: build(PIECES) }), beside())?.tasks?.map((one) => one.state)).toEqual(["done", "done", "working", "open", "open"]);
  });

  it("are left off where the agent made none, or the Session ended", () => {
    expect(workOf(record({ build: build([]) }), beside())?.tasks).toBeUndefined();
    expect(workOf(record(), beside())?.tasks).toBeUndefined();
    expect(workOf(record({ build: build(PIECES), state: "ended" }), beside())?.tasks).toBeUndefined();
  });

  it("give way to the plan once one is recorded", () => {
    const work = workOf(record({ build: build(PIECES), plan: PLAN }), beside());
    expect(work?.tasks).toBeUndefined();
    expect(work?.plan?.groups[0]?.tasks).toHaveLength(2);
  });

  it("reach the Session the view draws", () => {
    const session = sessionOfRecord(record({ build: build(PIECES) }), undefined, { picture: () => undefined, sketches: [], pending: [], workflows: [FEATURE] } as Beside);
    expect(session.work?.tasks?.[2]).toEqual({ id: "3", subject: "Rename them", state: "working" });
  });
});
