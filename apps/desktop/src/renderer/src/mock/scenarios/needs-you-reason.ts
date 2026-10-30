// Three Jobs that need him for different reasons, a command to run, a question and a walk to review, and one running. Each pulses its
// row and holds an amber pillar on the map; the walk `needs-you-reason` plays it, and `?take=row|pillar|both` is where the reason is
// drawn: a second line under the row's name, a label over the pillar, or both.

import type { TimelineEvent, TimelineSubject, WorktreeSlot, WorktreesHeld } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";
import { repository } from "@armada/screens/src/fixtures/repository";

import { mockAreas } from "../cockpit-areas";
import { holding } from "../holding";
import type { Scenario } from "../moment";
import { walkCall } from "../plan-call";
import { sessionsStore } from "../sessions/script";

const ARMADA = repository().manifest!.id;
const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const MIGRATE = "nr-migrate";
const RING = "nr-ring";
const DOCK = "nr-dock";
const CLOCK = "nr-clock";

const said = (id: string, text: string) => ({ id, at: "14:02:00", kind: "message", from: { kind: "agent" }, text }) as const;

const migrate: Session = {
  id: MIGRATE,
  title: "Migrate the payments store",
  manifestId: ARMADA,
  turn: { state: "idle" },
  lastTurnAt: ago(4),
  rows: [said("m1", "The migration needs to run against the sandbox database.")],
  attachments: [{ kind: "branch", name: "armada/payments-migrate", slot: 1 }],
  waitingFor: [{ id: "perm:migrate", text: "pnpm db:migrate --env sandbox", since: ago(4), source: "permission", options: [{ label: "Allow once" }, { label: "Refuse" }] }],
};

const ring: Session = {
  id: RING,
  title: "Draw the build ring",
  manifestId: ARMADA,
  turn: { state: "idle" },
  lastTurnAt: ago(7),
  rows: [said("r1", "Two ways to draw it.")],
  attachments: [{ kind: "branch", name: "armada/build-ring", slot: 2 }],
  waitingFor: [{ id: "ask:ring:0", text: "Which ring should a build draw?", since: ago(7), source: "ask_card", options: [{ label: "One ring" }, { label: "A ring a step" }] }],
};

const dock: Session = {
  id: DOCK,
  title: "Rebuild the dock panel",
  manifestId: ARMADA,
  turn: { state: "idle" },
  lastTurnAt: ago(11),
  rows: [said("d1", "The dock is rebuilt. A walk is ready.")],
  attachments: [{ kind: "branch", name: "armada/dock-panel", slot: 3 }],
  waitingFor: [walkCall("http://localhost:5391/?walk=dock-panel", "Review the dock panel walk", ago(11))],
};

const clock: Session = {
  id: CLOCK,
  title: "Pin the store clock",
  manifestId: ARMADA,
  turn: { state: "working" },
  lastTurnAt: ago(1),
  standing: "working",
  rows: [{ id: "c1", at: "14:00:00", kind: "message", from: { kind: "you" }, text: "Pin the clock in the store tests." }],
  attachments: [{ kind: "branch", name: "fix/pin-store-clock", slot: 4 }],
};

const all = [migrate, ring, dock, clock];

const bay = (one: Session, slot: number, branch: string): WorktreeSlot => ({
  manifest_id: ARMADA,
  slot,
  path: `/Users/user/armada/.armada/slots/slot-${slot}`,
  base: "main",
  warm: true,
  behind: 0,
  held: { state: "hosted_session", session_id: one.id, session_title: one.title! },
  branch,
  since: ago(60),
});

const pool = (): WorktreesHeld => ({ slots: [bay(migrate, 1, "armada/payments-migrate"), bay(ring, 2, "armada/build-ring"), bay(dock, 3, "armada/dock-panel"), bay(clock, 4, "fix/pin-store-clock")] });

let count = 0;
const who = (one: Session): { subject: TimelineSubject; title: string } => ({ subject: { kind: "session", id: one.id }, title: one.title! });
const event = (minutes: number, one: Session, kind: TimelineEvent["kind"], tone: TimelineEvent["tone"], says: string, rest: { detail?: string; items?: string[]; live?: boolean } = {}): TimelineEvent => {
  count += 1;
  const { subject, title } = who(one);
  return { id: count, at: ago(minutes), kind, subject, title, tone, says, manifest_id: ARMADA, ...rest };
};
const edited = (minutes: number, one: Session, items: string[]) => event(minutes, one, "session.edited", "run", `Edited ${items.length} files`, { detail: items[0]!, items, live: true });

/** What places the drones: where each has edited, and who holds each bay. */
const placed = (): TimelineEvent[] => [
  ...all.map((one, i) => event(60 - i, one, "session.started", "run", "Started", { live: true })),
  edited(20, migrate, ["crates/store/tests/clock.rs", "crates/store/tests/pool.rs"]),
  edited(15, ring, ["apps/desktop/src/renderer/src/cockpit/deck/ring.tsx", "apps/desktop/src/renderer/src/cockpit/deck/build.ts"]),
  edited(12, dock, ["apps/desktop/src/renderer/src/cockpit/dock/Page.tsx", "apps/desktop/src/renderer/src/cockpit/dock/keys.ts"]),
  edited(3, clock, ["crates/fleet/src/lease.rs", "crates/ipc/src/door.rs"]),
  ...all.map((one, i) => event(60 - i, one, "slot.leased", "idle", `Slot ${i + 1} leased`, { detail: `slot-${i + 1}` })),
];

function build(): Scenario {
  const base = holding("needs-you-reason", "Three Jobs that need him for a command, a question and a walk, and one running, with the reason on the row, over the pillar, or both");
  return {
    ...base,
    state: { ...base.state, repository: null },
    held: pool(),
    draft: { sessions: () => sessionsStore(all), events: placed(), areas: mockAreas },
  };
}

export const needsYouReason: Scenario = build();
