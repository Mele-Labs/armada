// The Job that rotates the audit log, for the scenarios `field-expand-plan` and `job-walk-on-the-spine`: five groups, one done, three under way, one not begun,
// and drones over the files they write. It also holds what the Job has made, for Done: two pull requests (one merged, one in the queue), two documents,
// two issues and a sketch (`auditLedger`), and the walks it shows and he approves (`walkActs`). Not a scenario: `scenarios/` takes one export a file.

import type { Session, SessionAttachment, SessionRow, WorkFile, WorkGroup } from "@armada/screens/src/draft/sessions";
import type { WaitingItem } from "@armada/protocol";
import { mockPage, openWalkWindow } from "@armada/screens/src/fixtures/walk-window";

import { walkCall, withWalkCall } from "./plan-call";
import { arrow, box, scene } from "./scenes";
import type { SessionsStore } from "./sessions/script";

export const BUSY = "one-view-3";

const GROUPS: readonly WorkGroup[] = [
  {
    id: "f1",
    agent: "rust-engineer",
    title: "Rotate at midnight",
    issue: "2931",
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
    issue: "2936",
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

const GIT = "https://git.example/armada/";

/** What the Job made along the way: two issues, two pull requests, a critique, a sketch. The plan, once approved, is the second document. */
const LEDGER: readonly SessionAttachment[] = [
  { kind: "issue", number: "2931", title: "Rotate the audit log at midnight", address: GIT + "issues/2931", state: "closed", pull_requests: [2934] },
  { kind: "issue", number: "2936", title: "Document the audit log rotation", address: GIT + "issues/2936", state: "open" },
  { kind: "pull_request", number: 2934, title: "Rotate the audit log at midnight", branch: "armada/audit-rotation", address: GIT + "pull/2934", checks: { state: "passed" }, state: "merged", auto: false, closes: [2931] },
  { kind: "pull_request", number: 2938, title: "Document the audit log rotation", branch: "armada/audit-rotation-docs", address: GIT + "pull/2938", checks: { state: "passed" }, state: "open", auto: true, queued: true, closes: [2936] },
  { kind: "artifact", form: "doc", id: "https://docs.example/d/audit-rotation-critique", title: "Audit rotation critique" },
  {
    kind: "sketch",
    id: "sketch:Audit file lifecycle",
    title: "Audit file lifecycle",
    by: "agent",
    drawing: scene(
      [box("live", 0, 0, "Audit log", "Written all day", { icon: "hard-drive", step: 1 }), box("rotated", 360, 0, "Rotated file", "Closed at midnight, compressed", { icon: "server", step: 2 }), box("pruned", 720, 0, "Pruned", "Past seven files", { icon: "globe", step: 3 })],
      [arrow("live", "rotated", "midnight", undefined, 2), arrow("rotated", "pruned", "keep count", undefined, 3)],
    ),
  },
];

/** The address a walk of this scenario's page is shown at. One page each, told apart by the take. */
export const walkPage = (scenario: string, take: string): string => {
  const url = new URL(mockPage());
  url.search = `?scenario=${scenario}&walked&page=${take}`;
  return url.toString();
};

/** A walk shown and waiting, or one he has approved. The rows are the thread's own: when it was shown, and when it ended. */
export const shown = (id: string, at: string, title: string, url: string, takes: number): SessionRow => ({ id, at, kind: "window", title, url, takes });
export const approved = (row: SessionRow, ended: string): SessionRow => (row.kind === "window" ? { ...row, ended } : row);

export const ROTATION = shown("b-3", "13:55:00", "Rotation schedule", walkPage("field-expand-plan", "rotation"), 2);
export const AUDIT_FILES = shown("b-4", "14:01:00", "Audit files in System", walkPage("field-expand-plan", "audit-files"), 3);

/** The takes the walk in `AUDIT_FILES` shows, one page each. */
export const AUDIT_TAKES = ["Table", "Cards", "List"] as const;

/** The Job at a step of its workflow: its plan, its field, and what it has made. `rows` are the walks it has shown. */
export const auditLedger = (one: Session, flow: { steps: string[]; at: number }, rows: readonly SessionRow[], waiting: readonly WaitingItem[] = []): Session => ({
  ...one,
  rows: [...one.rows, ...rows],
  attachments: [...one.attachments, ...LEDGER],
  waitingFor: [...(one.waitingFor ?? []), ...waiting],
  work: {
    workflow: { name: "Feature", steps: flow.steps, at: flow.at },
    proposal: { workflow: "Feature" },
    plan: { scope: ["crates/store", "crates/ipc", "crates/fleet", "packages/protocol", "apps/desktop", "docs/practices"], approved: true, groups: GROUPS },
    field: [
      { id: "main", path: "crates/store", doing: "read", file: STORE + "audit.rs" },
      { id: "protocol", agent: "protocol-engineer", task: "Mirror it in the TypeScript types", path: "packages/protocol", doing: "write", file: PROTOCOL + "audit.ts" },
      { id: "bridge", agent: "bridge-engineer", task: "List rotated files in System", path: "apps/desktop", doing: "write", file: SYSTEM + "AuditFiles.tsx" },
    ],
    files: FILES,
  },
});

/**
 * Opens a shown walk the way Bridge opens one, as a window over the app with its takes, and ends it on his Approve: the thread row ends, the item Fleet held for the page goes,
 * and `then` moves the workflow on. A press on the walk in Now, or Enter on it, is `draft.openWindow`, which this answers.
 */
export function walkActs(store: SessionsStore, id: string, walk: SessionRow, takes: readonly string[], then?: (one: Session) => Session): SessionsStore {
  if (walk.kind !== "window") return store;
  const { url, title } = walk;
  return {
    ...store,
    openWindow: (to, at) => {
      if (to !== id || at !== url) return store.openWindow?.(to, at);
      openWalkWindow(title, url, {
        title,
        job: "Rotate the audit log",
        into: "Session",
        onNote: () => undefined,
        variations: takes.map((name) => ({ name, url: `${url}&take=${name.toLowerCase()}` })),
        readiness: "ready",
        onApprove: async () => {
          store.edit(id, (one) => {
            const next = then === undefined ? one : then(one);
            return { ...next, rows: next.rows.map((row) => (row.kind === "window" && row.url === url ? approved(row, "2026-10-26T14:06:00.000Z") : row)), waitingFor: withWalkCall(next.waitingFor, url) };
          });
          return { ok: true };
        },
      });
    },
  };
}

/** The item Fleet holds for a shown walk, until his Approve. */
export const waitingOn = (row: SessionRow): WaitingItem[] => (row.kind === "window" ? [walkCall(row.url, row.title, "2026-10-26T14:01:00.000Z")] : []);
