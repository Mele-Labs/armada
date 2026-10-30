// A Job whose plan has eight groups, seven of them handed to Jobs of their own in seven different states, and the groups still shut. The Job view's spine draws each
// shut row with its Job on it. The group handed to the Job still working has every task ticked, as an agent ticks them when it hands them, and still reads
// as being worked. Two more Jobs were started outside the plan: one running, in Now, and one ended, in Done. Mock only: every field here is one Fleet would owe (`packages/screens/src/draft/sessions.ts`).

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { mockAreas } from "../cockpit-areas";
import type { Session, WorkGroup } from "@armada/screens/src/draft/sessions";

import { ARMADA, agent, ago, child, pull, started, task, tool, you } from "./linked";
import { sessionsStore } from "./script";

const PARENT = "jobs-1";
const OWNER = "armada";

type Handed = { id: string; group: string; title: string; slot: number; branch: string; change: (one: Session) => Session };

const NAMES = ["Name the migrations", "Apply them in order", "Record each once", "List them in System", "Mark the pending ones", "Roll one back", "Lock while applying", "Explain the order in the docs"];

const groups: WorkGroup[] = NAMES.map((title, at) => ({
  id: `g${at + 1}`,
  agent: at === 3 || at === 4 ? "bridge-engineer" : "rust-engineer",
  title,
  paths: [at === 3 || at === 4 ? "apps/desktop" : "crates/store"],
  tasks: [task(`T${at * 2 + 1}`, `${title}, first`, at === 2 || at === 5 ? "done" : "open"), task(`T${at * 2 + 2}`, `${title}, second`, at === 2 || at === 5 ? "done" : "open")],
}));

const BEFORE = "jobs-2";
const NOT_STARTED = "jobs-3";
const WORKING = "jobs-4";
const ASKING = "jobs-5";
const CHECKING = "jobs-6";
const MERGED = "jobs-7";
const ENDED = "jobs-8";
const OUTSIDE_RUNNING = "jobs-10";
const OUTSIDE_ENDED = "jobs-11";

/** Jobs its agent started outside the plan, with no group. */
const outside: Omit<Handed, "group">[] = [
  {
    id: OUTSIDE_RUNNING,
    title: "Seed the test database",
    slot: 10,
    branch: "armada/jobs-seed",
    change: (one) => ({ ...one, attachments: [...one.attachments, pull(2436, "Seed the test database", "armada/jobs-seed", { state: "open", checks: { state: "pending" } })] }),
  },
  { id: OUTSIDE_ENDED, title: "Check an old dump restores", slot: 11, branch: "armada/jobs-restore", change: (one) => ({ ...one, turn: { state: "idle" }, standing: "idle", dead: "ended", lastTurnAt: ago(30) }) },
];

const handed: Handed[] = [
  { id: BEFORE, group: "g1", title: "Migrations get names", slot: 3, branch: "armada/jobs-names", change: (one) => ({ ...one, turn: { state: "idle" }, standing: "idle", waits: { on: { id: WORKING, title: "Migrations apply in order" }, when: "ends", base: "main", needs: "", brief: "Name the migrations.", by: "agent" } }) },
  { id: NOT_STARTED, group: "g2", title: "Apply them in a fixed order", slot: 4, branch: "armada/jobs-order", change: (one) => ({ ...one, turn: { state: "idle" }, standing: "idle", start: { asksDelivery: false } }) },
  { id: WORKING, group: "g3", title: "Migrations apply in order", slot: 5, branch: "armada/jobs-record", change: (one) => one },
  {
    id: ASKING,
    group: "g4",
    title: "Applied migrations in System",
    slot: 6,
    branch: "armada/jobs-system",
    change: (one) => ({ ...one, asked: { command: "Show pending migrations too?", call: "jobs-q1", offers: ["allow_once", "refuse"], questions: [{ question: "Show pending migrations too?", header: "System", multi_select: false, options: [{ label: "Yes", description: "Pending ones draw in the same list" }, { label: "No", description: "Applied ones only" }] }] } }),
  },
  {
    id: CHECKING,
    group: "g5",
    title: "Pending migrations marked",
    slot: 7,
    branch: "armada/jobs-pending",
    change: (one) => ({ ...one, turn: { state: "idle" }, standing: "idle", attachments: [...one.attachments, pull(2431, "Mark the pending migrations", "armada/jobs-pending", { state: "open", checks: { state: "pending" } })] }),
  },
  {
    id: MERGED,
    group: "g6",
    title: "Roll one migration back",
    slot: 8,
    branch: "armada/jobs-rollback",
    change: (one) => ({
      ...one,
      turn: { state: "idle" },
      standing: "done",
      done: { since: ago(20) },
      attachments: [...one.attachments, pull(2427, "Roll one migration back", "armada/jobs-rollback", { state: "merged", checks: { state: "passed" } })],
    }),
  },
  { id: ENDED, group: "g7", title: "Lock while applying", slot: 9, branch: "armada/jobs-lock", change: (one) => ({ ...one, turn: { state: "idle" }, standing: "idle", dead: "ended" }) },
];

const parent = (): Session => ({
  id: PARENT,
  title: "Move the store to named migrations",
  manifestId: OWNER,
  cwd: ARMADA.root,
  model: "opus",
  effort: "high",
  mode: "auto",
  turn: { state: "idle" },
  lastTurnAt: ago(2),
  standing: "idle",
  rows: [
    you("j-1", "13:15:00", "Move the store to named migrations. Hand seven groups of the plan to Jobs of their own."),
    agent("j-2", "13:40:10", "Eight groups. The docs group stays with me."),
    tool("j-3", "13:40:20", "record_plan"),
    you("j-4", "13:42:00", "Approved."),
    ...handed.flatMap((one, at): Session["rows"] => [tool(`j-s${at}`, "13:42:30", `start_another_session ${one.title}`), started(`j-t${at}`, "13:42:31", one.id, one.title)]),
    you("j-5", "13:44:00", "Also seed the test database, and check an old dump still restores."),
    ...outside.flatMap((one, at): Session["rows"] => [tool(`j-o${at}`, "13:44:30", `start_another_session ${one.title}`), started(`j-u${at}`, "13:44:31", one.id, one.title)]),
  ],
  attachments: [
    { kind: "slot", slot: 2 },
    { kind: "branch", name: "armada/named-migrations", slot: 2 },
    pull(2400, "Move the store to named migrations", "armada/named-migrations", { state: "open", checks: { state: "pending" } }),
    { kind: "link", url: "https://git.example/armada/docs/migrations", by: "agent" },
    ...handed.map((one) => ({ kind: "started_to" as const, id: one.id, group: one.group })),
    ...outside.map((one) => ({ kind: "started_to" as const, id: one.id })),
  ],
  work: { railOff: true, plan: { scope: ["crates/store", "apps/desktop"], approved: true, groups } },
});

const other = (): Session => ({
  id: "jobs-9",
  title: "Rotate the audit log",
  manifestId: OWNER,
  cwd: ARMADA.root,
  turn: { state: "idle" },
  lastTurnAt: ago(50),
  standing: "idle",
  rows: [agent("o-1", "13:10:00", "The audit log rotates at midnight.")],
  attachments: [{ kind: "branch", name: "armada/audit-rotation", slot: 1 }],
});

export function linkedJobsOnPlan(name: string, says: string): Scenario {
  const board = onBoard({ repositories: [ARMADA], picked: ARMADA.root });
  const sessions = () =>
    sessionsStore([
      parent(),
      ...handed.map((one) => one.change(child(one.id, one.title, one.group, one.slot, one.branch, `${one.title}. Group: ${groups.find((g) => g.id === one.group)?.title ?? ""}.`, [], PARENT))),
      ...outside.map((one) => one.change(child(one.id, one.title, undefined, one.slot, one.branch, `${one.title}.`, [], PARENT))),
      other(),
    ]);
  return { ...board, name, says, draft: { sessions, areas: mockAreas } };
}
