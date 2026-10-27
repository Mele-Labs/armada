// A Job mid-build whose plan holds a task Fleet marked done itself and a task in progress that looks left behind, for the walk `taskMarkLook`. Over the
// `one-session-view` scenario, with its busy Session given the plan and the Field.

import type { SessionPlanTask } from "@armada/protocol";
import type { Session, WorkFile, WorkGroup, WorkTask } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s261OneSessionView as base } from "./one-session-view";

const BUSY = "one-view-3";

/** Twelve minutes before the scenario loads, which is when the stale task was last touched. */
const TOUCHED = new Date(Date.now() - 12 * 60_000).toISOString();

/** A task as Fleet puts it on the wire: the screens' `WorkTask` does not carry `marked` and `stale` yet, so this scenario hands them over beside it. */
type Task = WorkTask & Pick<SessionPlanTask, "marked" | "stale">;
const group = (id: string, agent: string, title: string, paths: string[], tasks: Task[]): WorkGroup => ({ id, agent, title, paths, tasks });

const GROUPS: readonly WorkGroup[] = [
  group("t1", "rust-engineer", "Rotate at midnight", ["crates/store"], [
    { id: "A1", text: "Rotate the log file at midnight", state: "done", seen: true },
    { id: "A2", text: "Keep seven files", state: "done", seen: true, marked: "commit 1a2b3c4 names it" },
  ]),
  group("t2", "protocol-engineer", "Report the rotation", ["packages/protocol"], [
    { id: "B1", text: "Add rotated_at to the audit reply", state: "working", seen: true },
    { id: "B2", text: "Mirror it in the TypeScript types", state: "working", seen: true, stale: { why: "edits went to Bridge", since: TOUCHED } },
  ]),
  group("t3", "bridge-engineer", "Show the rotation", ["apps/desktop"], [
    { id: "C1", text: "List rotated files in System", state: "working", seen: true },
    { id: "C2", text: "Open a rotated file from the list", state: "open" },
  ]),
];

const STORE = "crates/store/src/";
const SYSTEM = "apps/desktop/src/renderer/src/system/";

const FILES: readonly WorkFile[] = [{ path: STORE + "rotate.rs", act: "write" }, { path: STORE + "audit.rs" }, { path: SYSTEM + "AuditFiles.tsx", act: "write" }, { path: SYSTEM + "System.tsx" }];

const building = (one: Session): Session => ({
  ...one,
  work: {
    workflow: { name: "Feature", steps: ["Plan", "Build", "Verify", "Pull request"], at: 1 },
    plan: { scope: ["crates/store", "packages/protocol", "apps/desktop"], approved: true, groups: GROUPS },
    field: [{ id: "bridge", agent: "bridge-engineer", task: "List rotated files in System", path: "apps/desktop", doing: "write", file: SYSTEM + "AuditFiles.tsx" }],
    files: FILES,
  },
});

export const s332TaskMarkLook: Scenario = {
  ...base,
  name: "task-mark-look",
  says: "A Job mid-build: one task Fleet marked done itself, one in progress and left behind",
  draft: {
    ...base.draft,
    sessions: (control) => {
      const store = base.draft!.sessions!(control);
      store.edit(BUSY, building);
      return store;
    },
  },
};
