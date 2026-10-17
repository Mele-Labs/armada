// A Job mid-build with its plan in progress, for the walk `fieldExpandPlan`: five groups, one done, three under way, one not begun, and
// drones over the files they write. Over the `one-session-view` scenario, with its busy Session given the plan and the Field.

import type { Session, WorkFile, WorkGroup } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s261OneSessionView as base } from "./one-session-view";

const BUSY = "one-view-3";

const GROUPS: readonly WorkGroup[] = [
  {
    id: "f1",
    agent: "rust-engineer",
    title: "Rotate at midnight",
    paths: ["crates/store"],
    tasks: [
      { id: "A1", text: "Rotate the log file at midnight", state: "done", seen: true },
      { id: "A2", text: "Keep seven files", state: "done", seen: true },
      { id: "A3", text: "Compress rotated files", state: "done", seen: true },
    ],
  },
  {
    id: "f2",
    agent: "protocol-engineer",
    title: "Report the rotation",
    paths: ["crates/ipc", "packages/protocol"],
    tasks: [
      { id: "B1", text: "Add rotated_at to the audit reply", state: "done", seen: true },
      { id: "B2", text: "Mirror it in the TypeScript types", state: "working", seen: true },
    ],
  },
  {
    id: "f3",
    agent: "bridge-engineer",
    title: "Show the rotation",
    paths: ["apps/desktop"],
    tasks: [
      { id: "C1", text: "List rotated files in System", state: "working", seen: true },
      { id: "C2", text: "Open a rotated file from the list", state: "open" },
      { id: "C3", text: "Cover the list with a test", state: "open" },
    ],
  },
  {
    id: "f4",
    agent: "rust-engineer",
    title: "Prune old files",
    paths: ["crates/fleet"],
    tasks: [
      { id: "D1", text: "Delete files past the keep count", state: "open" },
      { id: "D2", text: "Log what was pruned", state: "open" },
    ],
  },
  {
    id: "f5",
    agent: "claude",
    title: "Note it in the practice",
    paths: ["docs/practices"],
    tasks: [{ id: "E1", text: "Add the rotation to the audit doc", state: "done" }],
  },
];

const STORE = "crates/store/src/";
const IPC = "crates/ipc/src/";
const PROTOCOL = "packages/protocol/src/";
const SYSTEM = "apps/desktop/src/renderer/src/system/";
const FLEET = "crates/fleet/src/";
const DOCS = "docs/practices/";

const FILES: readonly WorkFile[] = [
  { path: STORE + "audit.rs", act: "edit" },
  { path: STORE + "rotate.rs", act: "write" },
  { path: STORE + "compress.rs" },
  { path: IPC + "audit.rs", act: "edit" },
  { path: IPC + "wire.rs" },
  { path: PROTOCOL + "audit.ts", act: "write" },
  { path: PROTOCOL + "index.ts" },
  { path: SYSTEM + "AuditFiles.tsx", act: "write" },
  { path: SYSTEM + "System.tsx" },
  { path: SYSTEM + "audit.css" },
  { path: FLEET + "lease.rs" },
  { path: FLEET + "pool.rs" },
  { path: DOCS + "audit.md", act: "edit" },
];

const building = (one: Session): Session => ({
  ...one,
  work: {
    workflow: { name: "Feature", steps: ["Plan", "Build", "Verify", "Pull request"], at: 1 },
    plan: { scope: ["crates/store", "crates/ipc", "crates/fleet", "packages/protocol", "apps/desktop", "docs/practices"], approved: true, groups: GROUPS },
    field: [
      { id: "main", path: "crates/store", doing: "read", file: STORE + "audit.rs" },
      { id: "protocol", agent: "protocol-engineer", task: "Mirror it in the TypeScript types", path: "packages/protocol", doing: "write", file: PROTOCOL + "audit.ts" },
      { id: "bridge", agent: "bridge-engineer", task: "List rotated files in System", path: "apps/desktop", doing: "write", file: SYSTEM + "AuditFiles.tsx" },
    ],
    files: FILES,
  },
});

export const s274FieldExpandPlan: Scenario = {
  ...base,
  name: "field-expand-plan",
  says: "A Job mid-build: its plan in progress along the Field, and the Field opened large over the stage",
  draft: {
    ...base.draft,
    sessions: (control) => {
      const store = base.draft!.sessions!(control);
      store.edit(BUSY, building);
      return store;
    },
  },
};
