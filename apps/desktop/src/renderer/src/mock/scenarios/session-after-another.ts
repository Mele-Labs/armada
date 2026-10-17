// A Session that waits on another and starts on its own. The Cockpit with two live Sessions, one of them (`Rotate the audit
// log`) at work on a plan, and the dispatch sheet's After row to say a new Session waits on it. The waiting Session holds its
// words and no slot; the Session it waits on is told in its thread; its agent then changes when the other starts, and a pull
// request pushed meets the condition, so it starts with the handoff as its first message. The Sessions are draft data
// (`packages/screens/src/draft/sessions.ts`), nothing here reaches Fleet. The walk `sessionAfterAnother` plays it.

import { featureRunning } from "@armada/jobs/fake";
import type { Session, SessionRow } from "@armada/screens/src/draft/sessions";

import { asRow, holding, servedFrom } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";
import type { SessionsStore } from "../sessions/script";

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const board = asRow(featureRunning(), 83, "migrate", "Order the store migrations");
const OWNER = board.manifests[0]!.id;
const ROOT = servedFrom(board.manifests[0]!).root;

/** The Session the new one waits on: the walk picks it by this id. */
const AUDIT = "after-audit";
const QUIET = "after-migrations";
const BRANCH = "armada/audit-rotation";
const PULL = 2388;

const you = (id: string, at: string, text: string): SessionRow => ({ id, at, kind: "message", from: { kind: "you" }, text });
const agent = (id: string, at: string, text: string): SessionRow => ({ id, at, kind: "message", from: { kind: "agent" }, text });

const audit: Session = {
  id: AUDIT,
  title: "Rotate the audit log",
  manifestId: OWNER,
  cwd: ROOT,
  turn: { state: "working" },
  lastTurn: "14:00",
  lastTurnAt: ago(2),
  standing: "working",
  rows: [
    you("a-1", "13:50:00", "Rotate the audit log at midnight and keep seven files."),
    { id: "a-2", at: "13:58:12", kind: "tool", text: "Edit crates/store/src/audit.rs" },
    agent("a-3", "14:00:05", "Rotation and retention are in the store. The settings page is next."),
  ],
  attachments: [{ kind: "branch", name: BRANCH, slot: 1 }],
  work: {
    plan: {
      scope: ["crates/store", "apps/desktop"],
      approved: true,
      groups: [
        { id: "g1", agent: "rust-engineer", title: "Rotate at midnight", paths: ["crates/store"], tasks: [{ id: "T1", text: "Rotate the log file at midnight", state: "done", seen: true }, { id: "T2", text: "Keep seven files", state: "done", seen: true }] },
        { id: "g2", agent: "bridge-engineer", title: "Show the rotation", paths: ["apps/desktop"], tasks: [{ id: "T3", text: "List rotated files in System", state: "working", seen: true }] },
      ],
    },
  },
};

const quiet: Session = {
  id: QUIET,
  title: "Name the store migrations",
  manifestId: OWNER,
  cwd: ROOT,
  turn: { state: "idle" },
  lastTurn: "13:41",
  lastTurnAt: ago(22),
  standing: "idle",
  rows: [agent("q-1", "13:41:10", "The migrations are named by their order.")],
  attachments: [{ kind: "branch", name: "fix/migration-names", slot: 2 }],
};

function sessions(control: Parameters<typeof sessionsStore>[3]): SessionsStore {
  const none = { id: "x", number: 0, title: "", branch: "", slot: 0 };
  const base = sessionsStore([none, none], none, [], control, [], [audit, quiet], [], new Map([[ROOT, OWNER]]));
  let rowId = 0;
  let moment = 0;
  const said = (text: string): SessionRow => ({ id: `after-r${(rowId += 1)}`, at: `14:${String(10 + rowId).padStart(2, "0")}:00`, kind: "message", from: { kind: "agent" }, text });
  const waiting = () => base.get().find((one) => one.waits?.on.id === AUDIT);

  // What time brings, one moment each time a walk lets it pass: the agent of the Session being waited on changes when the
  // waiting one starts, and then pushes the pull request that meets the condition.
  const moments = [
    () => {
      const one = waiting();
      if (one === undefined) return;
      base.edit(AUDIT, (it) => ({ ...it, rows: [...it.rows, said(`Set ${one.title} to start when I push the pull request.`)] }));
      base.agentChanges(one.id, { when: "pushed" });
    },
    () => {
      const one = waiting();
      if (one === undefined) return;
      const address = `https://git.example/armada/pull/${PULL}`;
      base.edit(AUDIT, (it) => ({
        ...it,
        rows: [...it.rows, { id: "after-pr", at: "14:14:00", kind: "tool", text: "gh pr create --base main" }, said(`Opened #${PULL} on ${BRANCH}.`)],
        attachments: [...it.attachments, { kind: "pull_request", number: PULL, title: "Rotate the audit log", branch: BRANCH, address, checks: { state: "pending" }, state: "open", auto: false }],
      }));
      base.edit(one.id, (it) => (it.waits === undefined ? it : { ...it, waits: { ...it.waits, pull: { number: PULL, branch: BRANCH } } }));
      base.starts(one.id, `Pull request #${PULL} was pushed`);
    },
  ];
  base.later = () => {
    moments[moment]?.();
    moment += 1;
  };
  return base;
}

function build(): Scenario {
  const rows = [{ ...board, job: { ...board.job, owner_manifest_id: OWNER, started_at: ago(30) } }];
  const base = holding("session-after-another", "A Session dispatched to wait on another: the After row, its own row in the Work list, the thread of the Session it waits on, and its start", rows);
  return {
    ...base,
    state: { ...base.state, repository: ROOT, jobs: rows.map((one) => one.job) },
    draft: { sessions },
  };
}

export const sessionAfterAnother: Scenario = build();
