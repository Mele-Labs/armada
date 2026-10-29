// A Job whose agent handed work to a subagent and waits on it: the Agent call is a row that names the subagent, its last
// call and how long it has run, and the two messages the person sent meanwhile are marked queued. Stop cuts it off and
// the messages are read. Over the `sessions` scenario. The walk `foreground-subagent` plays it. armada-d3h.1.

import type { Session, SessionRow, SubagentThread } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const JOB = "sfg1";
const SUBAGENT = "sa-resize";
const TASK = "Make the Job browser thread resizable";

const waiting: Session = {
  id: JOB,
  title: "Resizable Job browser thread",
  turn: { state: "working" },
  standing: "working",
  lastTurnAt: ago(14),
  work: { workflow: { name: "Feature", steps: ["Plan the change", "Implement", "Review the change"], at: 1 } },
  rows: [
    { id: "fg1", at: ago(16), kind: "message", from: { kind: "you" }, text: "The Job browser's thread is too narrow to read a diff. Make it resizable." },
    { id: "fg2", at: ago(15), kind: "message", from: { kind: "agent" }, text: "The thread is a fixed column beside the page. A bridge-engineer subagent will add the handle and run the job-browser tests." },
    {
      id: "fg3",
      at: ago(14),
      kind: "tool",
      text: `Agent ${TASK}`,
      subagent: { id: SUBAGENT, agentType: "bridge-engineer", task: TASK, latest: "Bash pnpm exec vitest run src/renderer/src/job-browser", state: "running", since: ago(14) },
    },
    { id: "fg4", at: ago(6), kind: "message", from: { kind: "you" }, text: "Hello?", queued: true },
    { id: "fg5", at: ago(3), kind: "message", from: { kind: "you" }, text: "What are you doing?", queued: true },
  ],
  attachments: [
    { kind: "branch", name: "job-browser/resizable-thread", slot: 4 },
    { kind: "subagent", id: SUBAGENT, task: TASK, state: "running", agentType: "bridge-engineer", call: "fg3" },
  ],
};

/** The subagent's own thread so far: its brief, what it called, and no report yet. */
const SUBAGENT_ROWS: readonly SessionRow[] = [
  { id: "sr1", at: ago(14), kind: "message", from: { kind: "you" }, text: `${TASK}, with a drag handle like the side panel's.` },
  { id: "sr2", at: ago(13), kind: "tool", text: "Read apps/desktop/src/renderer/src/job-browser/JobBrowserPane.tsx" },
  { id: "sr3", at: ago(11), kind: "tool", text: "Edit apps/desktop/src/renderer/src/job-browser/JobBrowserPane.tsx" },
  { id: "sr4", at: ago(9), kind: "message", from: { kind: "agent" }, text: "The handle is in. Running the job-browser tests." },
  { id: "sr5", at: ago(8), kind: "tool", text: "Bash pnpm exec vitest run src/renderer/src/job-browser" },
];

/** What Stop leaves: the turn ended, the subagent cut off where it stood, and the held messages read. */
function stopped(one: Session): Session {
  const ended = new Date().toISOString();
  return {
    ...one,
    turn: { state: "idle" },
    standing: "idle",
    lastTurnAt: ended,
    rows: one.rows.map((row): SessionRow => {
      if (row.kind === "tool" && row.subagent?.state === "running") return { ...row, subagent: { ...row.subagent, state: "cut_off", ended } };
      if (row.kind === "message" && row.queued === true) {
        const { queued, ...read } = row;
        void queued;
        return read;
      }
      return row;
    }),
    attachments: one.attachments.map((held) => (held.kind === "subagent" && held.id === SUBAGENT ? { ...held, state: "done" as const } : held)),
  };
}

function sessionsOf(board: Parameters<NonNullable<NonNullable<Scenario["draft"]>["sessions"]>>[0]) {
  const store = s200Sessions.draft!.sessions!(board, [waiting]);
  return {
    ...store,
    stop: (id: string) => (id === JOB ? store.edit(id, stopped) : store.stop?.(id)),
    subagent: async (id: string, subagentId: string): Promise<SubagentThread | undefined> => {
      if (id !== JOB || subagentId !== SUBAGENT) return store.subagent?.(id, subagentId);
      return { rows: [...SUBAGENT_ROWS], finished: false, model: "vendor-opus-5-5", context: { tokens: 64_000, window: 200_000 } };
    },
  };
}

export const s290ForegroundSubagent: Scenario = {
  ...s200Sessions,
  name: "foreground-subagent",
  says: "A Job waiting on a subagent: its row names the subagent, its last call and its time, and messages sent meanwhile are queued",
  draft: { sessions: sessionsOf },
};
