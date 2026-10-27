// A walk is an output of the Job, as a pull request is, and this plays its whole life on the Job's story: pending in Next under the workflow's Present step; shown, it
// moves to Now as a call on him, a lamp that pulses, its name and a tick for each take; pressing it, or Enter on it, opens the walk window as Bridge opens one; and on
// his Approve it moves to Completed under Present as an approved walk. The walk's `later` step shows the walk. Over the same Job as `field-expand-plan`, at Build, with the real Job of
// `job-real-issues-tab` in place of a second.

import type { Session, SessionRow } from "@armada/screens/src/draft/sessions";

import { AUDIT_TAKES, BUSY, auditLedger, shown, waitingOn, walkActs, walkPage } from "../audit-job";
import type { Scenario } from "../moment";
import { asRealIssuesJob } from "./job-real-issues-tab";
import { s261OneSessionView as base } from "./one-session-view";

const FLOW = ["Draft", "Build", "Present", "Verify", "Pull request"];
const WALK = shown("b-4", "14:01:00", "Audit files in System", walkPage("job-walk-on-the-spine", "audit-files"), 3);

const say = (id: string, at: string, text: string): SessionRow => ({ id, at, kind: "message", from: { kind: "agent" }, text });

/** The thread over the Job's story: one group of calls for each kind the calls row names, the last one still running. */
const CALLS: readonly SessionRow[] = [
  { id: "c-1", at: "13:50:20", kind: "tool", text: "Read crates/store/src/audit.rs" },
  { id: "c-2", at: "13:50:24", kind: "tool", text: "Read crates/store/src/lib.rs" },
  say("c-3", "13:50:30", "The audit log is one file today."),
  { id: "c-4", at: "13:50:35", kind: "tool", text: "Grep rotate_at" },
  say("c-5", "13:50:40", "Nothing rotates it yet."),
  { id: "c-6", at: "13:51:02", kind: "tool", text: "Edit crates/store/src/audit.rs" },
  { id: "c-7", at: "13:51:09", kind: "tool", text: "Edit crates/store/src/lib.rs" },
  say("c-8", "13:51:20", "Rotation is in; running the checks."),
  { id: "c-9", at: "13:52:01", kind: "tool", text: "Bash cd /work && armada check test audit" },
  { id: "c-10", at: "13:52:30", kind: "tool", text: "Bash armada check test rotate" },
  { id: "c-11", at: "13:52:58", kind: "tool", text: "Bash cargo clippy -p store" },
  say("c-12", "13:53:10", "Green. A reviewer next."),
  { id: "c-13", at: "13:53:20", kind: "tool", text: "Agent Review the rotation" },
  say("c-14", "13:53:30", "Checking how others rotate at midnight."),
  { id: "c-15", at: "13:53:40", kind: "tool", text: "WebSearch rotating logs at midnight" },
  say("c-16", "13:53:50", "Pushed; the deploy is next."),
  { id: "c-17", at: "13:53:55", kind: "tool", text: "start_watch deploy" },
  say("c-18", "13:54:00", "One thing to settle with you."),
  { id: "c-19", at: "13:54:05", kind: "tool", text: "AskUserQuestion" },
  say("c-20", "13:54:10", "Mirroring it in the TypeScript types."),
  { id: "b-2", at: "14:00:05", kind: "tool", text: "Edit crates/store/src/audit.rs" },
];

/** The workflow at a step. */
const at = (step: number) => (one: Session): Session => (one.work?.workflow === undefined ? one : { ...one, work: { ...one.work, workflow: { ...one.work.workflow, at: step } } });

export const s332JobWalkOnTheSpine: Scenario = {
  ...base,
  name: "job-walk-on-the-spine",
  says: "A Job at Build whose workflow still has Present ahead: its walk is pending in Next, then shown in Now, then approved in Completed",
  draft: {
    ...base.draft,
    sessions: (control) => {
      const store = base.draft!.sessions!(control);
      store.edit(BUSY, (one) => {
        const ledger = auditLedger(one, { steps: FLOW, at: 1 }, []);
        return { ...ledger, rows: [...ledger.rows.filter((row) => row.kind === "message" && row.from.kind === "you"), ...CALLS] };
      });
      // A second Job beside it, the real one from `job-real-issues-tab`, so one walk shows Now on invented work and on real work.
      store.edit("one-view-4", asRealIssuesJob);
      let shownYet = false;
      return {
        ...walkActs(store, BUSY, WALK, AUDIT_TAKES, at(3)),
        // Time passes: Build is done and the Job shows the walk.
        later: () => {
          if (shownYet) return;
          shownYet = true;
          store.edit(BUSY, (one) => ({ ...at(2)(one), rows: [...one.rows, WALK], waitingFor: [...(one.waitingFor ?? []), ...waitingOn(WALK)] }));
        },
      };
    },
  },
};
