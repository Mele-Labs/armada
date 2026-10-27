// A real Job on the Job story: c2d23c4c, an Issues tab for Bridge, at tool call 396 of 676 (`real-issues-job.ts`). Seven groups and thirty-two tasks, ten of them
// marked in progress, two of those untouched for over an hour and a half and an hour; the thread ends in the calls the Job made, which Now reads its leading line from.
// Over `one-session-view`, in place of the Job that rotates the audit log.

import type { Session, SessionRow, WorkGroup, WorkTask, WorkTaskState } from "@armada/screens/src/draft/sessions";

import { REAL_CALLS, REAL_GROUPS, REAL_MARKS, REAL_SUMMARY, SNAPSHOT_AT } from "../real-issues-job";
import type { Scenario } from "../moment";
import { s261OneSessionView as base } from "./one-session-view";

const BUSY = "one-view-3";

/** About how long one tool call took in this Job, which turns "calls since touched" into a time. */
const SECONDS_A_CALL = 20;

const STATE: Record<string, WorkTaskState> = { in_progress: "working", completed: "done", pending: "open" };

/** The plan's groups as the Job recorded them, each task in the state its mark had. A mark in progress carries when it last changed, counted back from now. */
function groupsOf(at: number): WorkGroup[] {
  return REAL_GROUPS.map(
    (group): WorkGroup => ({
      id: group.id,
      agent: group.agent,
      title: group.title,
      gives: group.gives,
      paths: group.paths,
      tasks: group.tasks.map((task): WorkTask => {
        const mark = REAL_MARKS[task.id.slice(1)];
        const state = STATE[mark?.status ?? "pending"] ?? "open";
        return { id: task.id, text: task.title, state, ...(state === "working" && mark !== undefined ? { changedAt: new Date(at - mark.callsSinceTouched * SECONDS_A_CALL * 1000).toISOString() } : {}) };
      }),
    }),
  );
}

/** The thread: the last sixty calls, twenty seconds apart, ending at the moment of the snapshot. */
function callsOf(): SessionRow[] {
  const end = Date.parse(SNAPSHOT_AT);
  return REAL_CALLS.map(([tool, input], i): SessionRow => {
    const clock = new Date(end - (REAL_CALLS.length - 1 - i) * SECONDS_A_CALL * 1000).toISOString().slice(11, 19);
    return { id: `real-${i + 1}`, at: clock, kind: "tool", text: input === "" ? tool : `${tool} ${input}` };
  });
}

/** Any Job of the mock turned into the real one: its title, a turn that runs, its calls, and its plan with the marks it had. */
export const asRealIssuesJob = (one: Session): Session => {
  const groups = groupsOf(Date.now());
  return {
    ...one,
    title: "Issues tab beside Reviews",
    turn: { state: "working" },
    rows: callsOf(),
    attachments: [],
    work: { plan: { summary: REAL_SUMMARY, scope: [...new Set(groups.flatMap((group) => group.paths))], approved: true, groups } },
  };
};

export const s333JobRealIssuesTab: Scenario = {
  ...base,
  name: "job-real-issues-tab",
  says: "A real Job at call 396 of 676: seven groups, ten marks in progress, two of them untouched for an hour or more. Now leads with what its tool calls say it is doing",
  draft: {
    ...base.draft,
    sessions: (control) => {
      const store = base.draft!.sessions!(control);
      store.edit(BUSY, asRealIssuesJob);
      return store;
    },
  },
};
