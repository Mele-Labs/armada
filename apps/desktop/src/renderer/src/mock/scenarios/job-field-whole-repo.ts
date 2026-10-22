// A Job open with its Field drawing the whole repository, for the walk `jobFieldWholeRepo`: two other live Jobs in the same
// repository faded on it, the files this Job wrote and read glowing as they fade, and the replay slider along the Field's foot.
// The reads and edits are `session.read` and `session.edited` bursts at their own times, one of them spelled under the Job's slot.

import { repository } from "@armada/screens/src/fixtures/repository";
import type { TimelineEvent, WorktreeSlot, WorktreesHeld } from "@armada/protocol";
import type { Session, WorkFile } from "@armada/screens/src/draft/sessions";

import { mockAreas } from "../cockpit-areas";
import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

const ARMADA = repository().manifest!.id;
const ROOT = repository().root;
const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();
const slotPath = (slot: number) => `${ROOT}/.armada/slots/slot-${slot}`;

const STORE = "crates/store/src/";
const PROTOCOL = "packages/protocol/src/";
const DESKTOP = "apps/desktop/src/renderer/src/";

const FILES: readonly WorkFile[] = [
  { path: STORE + "audit.rs", act: "edit" },
  { path: STORE + "rotate.rs", act: "write" },
  { path: PROTOCOL + "audit.ts", act: "edit" },
];

const own: Session = {
  id: "jf-own",
  title: "Rotate the audit log",
  manifestId: ARMADA,
  turn: { state: "working" },
  lastTurnAt: ago(1),
  standing: "working",
  rows: [{ id: "jf-u", at: "09:00:00", kind: "message", from: { kind: "you" }, text: "Rotate the audit log at midnight and keep seven files." }],
  attachments: [{ kind: "branch", name: "armada/audit-rotation", slot: 1 }],
  work: {
    plan: {
      scope: ["crates/store", "packages/protocol"],
      approved: true,
      groups: [
        { id: "g1", agent: "rust-engineer", title: "Rotate at midnight", paths: ["crates/store"], tasks: [{ id: "T1", text: "Rotate the log file at midnight", state: "done", seen: true }, { id: "T2", text: "Keep seven files", state: "working", seen: true }] },
        { id: "g2", agent: "protocol-engineer", title: "Report the rotation", paths: ["packages/protocol"], tasks: [{ id: "T3", text: "Add rotated_at to the audit reply", state: "open" }] },
      ],
    },
    field: [
      { id: "main", path: "crates/store", doing: "write", file: STORE + "rotate.rs" },
      { id: "protocol", agent: "protocol-engineer", task: "Add rotated_at to the audit reply", path: "packages/protocol", doing: "write", file: PROTOCOL + "audit.ts" },
    ],
    files: FILES,
  },
};

/** Two other live Jobs in the same repository, each with its own drone over the file it is writing. */
const clock: Session = {
  id: "jf-clock",
  title: "Pin the store clock",
  manifestId: ARMADA,
  turn: { state: "working" },
  lastTurnAt: ago(2),
  standing: "working",
  rows: [{ id: "jc-u", at: "09:10:00", kind: "message", from: { kind: "you" }, text: "Pin the clock in the store tests." }],
  attachments: [{ kind: "branch", name: "fix/pin-store-clock", slot: 2 }],
  work: { field: [{ id: "main", path: "apps/desktop", doing: "write", file: DESKTOP + "cockpit/deck/clock.ts" }], files: [{ path: DESKTOP + "cockpit/deck/clock.ts", act: "edit" }] },
};

const lease: Session = {
  id: "jf-lease",
  title: "Retry the slot lease",
  manifestId: ARMADA,
  turn: { state: "working" },
  lastTurnAt: ago(3),
  standing: "working",
  rows: [{ id: "jl-u", at: "09:20:00", kind: "message", from: { kind: "you" }, text: "Retry the slot lease when the pool is contended." }],
  attachments: [{ kind: "branch", name: "fix/lease-retry", slot: 3 }],
  work: { field: [{ id: "main", path: "crates/fleet", doing: "write", file: "crates/fleet/src/lease.rs" }], files: [{ path: "crates/fleet/src/lease.rs", act: "edit" }] },
};

const bay = (slot: number, held: WorktreeSlot["held"], rest: Partial<WorktreeSlot> = {}): WorktreeSlot => ({ manifest_id: ARMADA, slot, path: slotPath(slot), base: "main", warm: true, behind: 0, held, ...rest });

function pool(): WorktreesHeld {
  return {
    slots: [
      bay(1, { state: "hosted_session", session_id: own.id, session_title: own.title! }, { branch: "armada/audit-rotation", since: ago(40) }),
      bay(2, { state: "hosted_session", session_id: clock.id, session_title: clock.title! }, { branch: "fix/pin-store-clock", since: ago(30) }),
      bay(3, { state: "hosted_session", session_id: lease.id, session_title: lease.title! }, { branch: "fix/lease-retry", since: ago(25) }),
    ],
  };
}

let count = 0;
function burst(minutes: number, session: Session, kind: "session.read" | "session.edited", items: string[]): TimelineEvent {
  count += 1;
  const says = `${kind === "session.read" ? "Read" : "Edited"} ${items.length} ${items.length === 1 ? "file" : "files"}`;
  return { id: count, at: ago(minutes), kind, subject: { kind: "session", id: session.id }, title: session.title!, tone: "run", says, manifest_id: ARMADA, detail: items[0]!, items, live: true };
}

/**
 * This Job's reads and edits over forty minutes. The newest are still glowing: a write fades over ten minutes and a read over five,
 * so the edits at one and six minutes and the reads at two and four are lit, and the older ones are on the slider and the route only.
 */
function events(): TimelineEvent[] {
  return [
    { id: ++count, at: ago(40), kind: "session.started", subject: { kind: "session", id: own.id }, title: own.title!, tone: "run", says: "Started", manifest_id: ARMADA, live: true },
    burst(36, own, "session.read", [STORE + "audit.rs", STORE + "lib.rs"]),
    burst(30, own, "session.read", ["crates/fleet/src/pool.rs"]),
    burst(24, own, "session.edited", [STORE + "audit.rs"]),
    burst(16, own, "session.read", [PROTOCOL + "index.ts"]),
    burst(6, own, "session.edited", [PROTOCOL + "audit.ts"]),
    burst(4, own, "session.read", ["docs/practices/audit.md"]),
    burst(2, own, "session.read", [STORE + "audit.rs"]),
    burst(1, own, "session.edited", [`${slotPath(1)}/${STORE}rotate.rs`]),
    burst(3, clock, "session.edited", [DESKTOP + "cockpit/deck/clock.ts"]),
    burst(5, lease, "session.edited", ["crates/fleet/src/lease.rs"]),
  ];
}

function build(): Scenario {
  const base = holding("job-field-whole-repo", "A Job open with its Field drawing the whole repository: two other Jobs faded, the files it wrote and read glowing, and the replay slider");
  return {
    ...base,
    state: { ...base.state, repository: null },
    held: pool(),
    draft: { sessions: () => sessionsStore([own, clock, lease], [], new Map(), []), events: events(), areas: mockAreas },
  };
}

export const s314JobFieldWholeRepo: Scenario = build();
