// A Session that split its work into linked Sessions it follows. The parent draws them on the Plan's groups and in a Ledger section called
// Linked, and the children stand under it in Work. Three of the plan's four groups were handed to Sessions of
// their own, and one of those started a Job of its own, which stands under it a step further in; each time a walk lets time pass one of
// them says something the parent is told. Mock only: Fleet carries no group on
// `start_another_session` yet, and every field here is one it would owe (`packages/screens/src/draft/sessions.ts`).

import type { RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session, SessionAttachment, SessionRow, WorkGroup, WorkTask } from "@armada/screens/src/draft/sessions";

import { mockAreas } from "../cockpit-areas";
import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { sessionsStore } from "./script";
import type { SessionsStore } from "./script";

export const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };
const OWNER = "armada";

const PARENT = "linked-1";
const NAMES = "linked-2";
const ORDER = "linked-3";
const RECORD = "linked-4";
const OTHER = "linked-5";
const SORT = "linked-6";

export const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();
export const you = (id: string, at: string, text: string): SessionRow => ({ id, at, kind: "message", from: { kind: "you" }, text });
export const agent = (id: string, at: string, text: string): SessionRow => ({ id, at, kind: "message", from: { kind: "agent" }, text });
export const tool = (id: string, at: string, text: string): SessionRow => ({ id, at, kind: "tool", text });

export const task = (id: string, text: string, state: WorkTask["state"]): WorkTask => ({ id, text, state, seen: state !== "open" });

/** The plan's groups, with the last of them still the parent's own. `merged` is the third group's pull request landing. */
const groups = (merged: boolean): WorkGroup[] => [
  { id: "g1", agent: "rust-engineer", title: "Name the migrations", paths: ["crates/store"], tasks: [task("T1", "Give each migration a name", "working"), task("T2", "Record the name with the version", "open")] },
  { id: "g2", agent: "rust-engineer", title: "Apply them in order", paths: ["crates/store"], tasks: [task("T3", "Sort by name before applying", "working"), task("T4", "Stop at the first failure", "open")] },
  { id: "g3", agent: "rust-engineer", title: "Record each once", paths: ["crates/store"], tasks: [task("T5", "Write one row for each applied migration", merged ? "done" : "working")] },
  { id: "g4", agent: "bridge-engineer", title: "List them in System", paths: ["apps/desktop"], tasks: [task("T6", "Show the applied migrations", "working"), task("T7", "Mark the pending ones", "open")] },
];

const reads = ["migrations.rs", "runner.rs", "version.rs", "apply.rs", "lock.rs", "audit.rs"];
const filler: SessionRow[] = reads.flatMap((file, at): SessionRow[] => [
  tool(`p-r${at}`, `13:${String(20 + at).padStart(2, "0")}:00`, `Read crates/store/src/${file}`),
  agent(`p-n${at}`, `13:${String(20 + at).padStart(2, "0")}:30`, `${file} is read by one caller. Its order comes from the version number.`),
]);

export const started = (id: string, at: string, session: string, title: string): SessionRow => ({ id, at, kind: "started", session: { id: session, title }, fork: false });

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
    you("p-1", "13:15:00", "Move the store to named migrations. Hand the groups of the plan to their own Sessions and keep track of them."),
    ...filler,
    agent("p-2", "13:40:10", "Four groups: names, order, record and the System page."),
    tool("p-3", "13:40:20", "record_plan"),
    you("p-4", "13:42:00", "Approved. The first three can go to their own Sessions."),
    tool("p-5", "13:42:30", "start_another_session Migrations get names"),
    started("p-6", "13:42:31", NAMES, "Migrations get names"),
    tool("p-7", "13:42:40", "start_another_session Migrations apply in order"),
    started("p-8", "13:42:41", ORDER, "Migrations apply in order"),
    tool("p-9", "13:42:50", "start_another_session One row for each migration"),
    started("p-10", "13:42:51", RECORD, "One row for each migration"),
    agent("p-11", "13:43:10", "Three Sessions started, one for each group. I am on the System page."),
    tool("p-12", "14:01:30", "Edit apps/desktop/src/renderer/src/system/Migrations.tsx"),
  ],
  attachments: [
    { kind: "slot", slot: 2 },
    { kind: "branch", name: "armada/named-migrations", slot: 2 },
    { kind: "started_to", id: NAMES, group: "g1" },
    { kind: "started_to", id: ORDER, group: "g2" },
    { kind: "started_to", id: RECORD, group: "g3" },
  ],
  work: { railOff: true, plan: { scope: ["crates/store", "apps/desktop"], approved: true, groups: groups(false) } },
});

export const pull = (number: number, title: string, branch: string, rest: Partial<Extract<SessionAttachment, { kind: "pull_request" }>>): SessionAttachment => ({
  kind: "pull_request",
  number,
  title,
  branch,
  address: `https://git.example/armada/pull/${number}`,
  checks: { state: "pending" },
  state: "draft",
  auto: false,
  ...rest,
});

export const child = (id: string, title: string, group: string | undefined, slot: number, branch: string, brief: string, more: SessionAttachment[], by = PARENT): Session => ({
  id,
  title,
  manifestId: OWNER,
  cwd: ARMADA.root,
  mode: "auto",
  turn: { state: "working" },
  lastTurnAt: ago(1),
  standing: "working",
  rows: [you(`${id}-1`, "13:42:31", brief), tool(`${id}-2`, "13:43:00", "Read crates/store/src/migrations.rs"), tool(`${id}-3`, "13:50:00", `Edit crates/store/src/${branch.split("/").at(-1)}.rs`)],
  attachments: [{ kind: "started_by", id: by, ...(group === undefined ? {} : { group }) }, { kind: "slot", slot }, { kind: "branch", name: branch, slot }, ...more],
});

const names = (): Session => child(NAMES, "Migrations get names", "g1", 3, "armada/migration-names", "Give each migration a name and record it with the version. Group: Name the migrations.", [pull(2412, "Name the migrations", "armada/migration-names", {})]);
const order = (): Session =>
  child(ORDER, "Migrations apply in order", "g2", 4, "armada/migration-order", "Apply the migrations sorted by name and stop at the first failure. Group: Apply them in order.", [{ kind: "started_to", id: SORT }]);
/** A Job the second child started: a grandchild of the parent. */
const sort = (): Session => child(SORT, "Sort names that share a prefix", undefined, 7, "armada/migration-sort", "Sort migration names that share a prefix by their number.", [], ORDER);
const record = (): Session =>
  child(RECORD, "One row for each migration", "g3", 5, "armada/migration-record", "Write one row for each applied migration. Group: Record each once.", [pull(2409, "Record each migration once", "armada/migration-record", { state: "open", checks: { state: "passed" } })]);

const other = (): Session => ({
  id: OTHER,
  title: "Rotate the audit log",
  manifestId: OWNER,
  cwd: ARMADA.root,
  turn: { state: "idle" },
  lastTurnAt: ago(50),
  standing: "idle",
  rows: [agent("o-1", "13:10:00", "The audit log rotates at midnight.")],
  attachments: [{ kind: "branch", name: "armada/audit-rotation", slot: 1 }],
});

const QUESTION = {
  question: "Sort by name or by version?",
  header: "Order",
  multi_select: false,
  options: [
    { label: "Name", description: "The names sort the same as the order they were written in" },
    { label: "Version", description: "Keeps the numbers as the order, and the name is a label" },
  ],
};

/** What the parent is told, as the row its thread keeps. */
const told = (id: string, at: string, session: string, title: string, event: "done" | "merged"): SessionRow => ({ id, at, kind: "linked", session: { id: session, title }, event });

function sessions(): SessionsStore {
  const base = sessionsStore([parent(), names(), order(), sort(), record(), other()]);
  let moment = 0;
  const handed: Session[] = [];
  let read: { from: readonly Session[]; made: number; out: readonly Session[] } | undefined;
  return {
    ...base,
    // Sessions handed a group after the window opened stand beside the ones it was built with.
    get: () => {
      const now = base.get();
      if (read === undefined || read.from !== now || read.made !== handed.length) read = { from: now, made: handed.length, out: [...now, ...handed] };
      return read.out;
    },
    // The owner hands a group to a Session of its own: the Session starts on it and the parent follows.
    handOff: (id, group) => {
      const from = base.get().find((one) => one.id === id);
      const title = from?.work?.plan?.groups.find((one) => one.id === group)?.title;
      if (from === undefined || title === undefined) return undefined;
      const session = `linked-hand-${handed.length + 1}`;
      handed.push(child(session, title, group, 6 + handed.length, `armada/handed-${handed.length + 1}`, `${title}. Group: ${title}.`, []));
      base.edit(id, (one) => ({
        ...one,
        rows: [...one.rows, tool(`p-h${handed.length}a`, "14:06:00", `start_another_session ${title}`), started(`p-h${handed.length}b`, "14:06:01", session, title)],
        attachments: [...one.attachments, { kind: "started_to", id: session, group }],
      }));
      return session;
    },
    later() {
      moment += 1;
      if (moment === 1) {
        // One of them asks the owner a question. It is his call, on its own row, and the parent is not told.
        base.edit(ORDER, (one) => ({ ...one, asked: { command: QUESTION.question, call: "linked-q1", offers: ["allow_once", "refuse"], questions: [QUESTION] }, lastTurnAt: new Date().toISOString() }));
      } else if (moment === 2) {
        // Another one's pull request merges and the Session is done.
        base.edit(RECORD, (one) => ({
          ...one,
          turn: { state: "idle" },
          standing: "done",
          done: { since: new Date().toISOString() },
          lastTurnAt: new Date().toISOString(),
          attachments: one.attachments.map((a) => (a.kind === "pull_request" ? { ...a, state: "merged" as const } : a)),
        }));
        base.edit(PARENT, (one) => ({ ...one, rows: [...one.rows, told("p-l2", "14:05:40", RECORD, "One row for each migration", "merged")], work: { railOff: true, plan: { ...one.work!.plan!, groups: groups(true) } } }));
      }
    },
  };
}

export function linkedSessions(name: string, says: string): Scenario {
  const board = onBoard({ repositories: [ARMADA], picked: ARMADA.root });
  return { ...board, name, says, draft: { sessions, areas: mockAreas } };
}
