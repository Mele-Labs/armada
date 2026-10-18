// A Job mid-build whose plan holds tasks the agent's list marks completed with no edit seen, for the walk `handedInReadsDone`. Over the
// `one-session-view` scenario, with its busy Session given the plan and the Field.

import type { Session, WorkFile, WorkGroup } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s261OneSessionView as base } from "./one-session-view";

const BUSY = "one-view-3";

const GROUPS: readonly WorkGroup[] = [
  {
    id: "h1",
    agent: "rust-engineer",
    title: "Rotate at midnight",
    paths: ["crates/store"],
    tasks: [
      { id: "A1", text: "Read how the store names its files", state: "handed_in" },
      { id: "A2", text: "Rotate the log file at midnight", state: "done", seen: true },
      { id: "A3", text: "Run the store tests", state: "handed_in" },
    ],
  },
  {
    id: "h2",
    agent: "bridge-engineer",
    title: "Show the rotation",
    paths: ["apps/desktop"],
    tasks: [
      { id: "B1", text: "List rotated files in System", state: "working", seen: true },
      { id: "B2", text: "Open a rotated file from the list", state: "open" },
    ],
  },
];

const STORE = "crates/store/src/";
const SYSTEM = "apps/desktop/src/renderer/src/system/";

const FILES: readonly WorkFile[] = [{ path: STORE + "rotate.rs", act: "write" }, { path: STORE + "audit.rs" }, { path: SYSTEM + "AuditFiles.tsx", act: "write" }, { path: SYSTEM + "System.tsx" }];

const building = (one: Session): Session => ({
  ...one,
  work: {
    workflow: { name: "Feature", steps: ["Plan", "Build", "Verify", "Pull request"], at: 1 },
    plan: { scope: ["crates/store", "apps/desktop"], approved: true, groups: GROUPS },
    field: [{ id: "bridge", agent: "bridge-engineer", task: "List rotated files in System", path: "apps/desktop", doing: "write", file: SYSTEM + "AuditFiles.tsx" }],
    files: FILES,
  },
});

export const s290HandedInReadsDone: Scenario = {
  ...base,
  name: "handed-in-reads-done",
  says: "A Job mid-build: two tasks the agent's list marks completed with no edit seen, drawn as done",
  draft: {
    ...base.draft,
    sessions: (control) => {
      const store = base.draft!.sessions!(control);
      store.edit(BUSY, building);
      return store;
    },
  },
};
