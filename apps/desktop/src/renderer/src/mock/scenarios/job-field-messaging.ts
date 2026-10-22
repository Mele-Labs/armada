// A Job asking another Job of its repository a question on its Field, for the walk `jobFieldMessaging`: it sends, the other replies, and
// the arc goes stale. Each message is the `session.messaged` entry Fleet writes when it delivers one, the sender as the subject and the
// receiver as `to`, dated when the walk's `later` step lets it come; the last step lets the time a message stays fresh pass.

import { repository } from "@armada/screens/src/fixtures/repository";
import type { TimelineEvent, WorktreeSlot, WorktreesHeld } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";

import { mockAreas } from "../cockpit-areas";
import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

const ARMADA = repository().manifest!.id;
const ROOT = repository().root;
const ago = (seconds: number) => new Date(Date.now() - seconds * 1000).toISOString();

const STORE = "crates/store/src/";

const session = (id: string, title: string, branch: string, slot: number, path: string, file: string): Session => ({
  id,
  title,
  manifestId: ARMADA,
  turn: { state: "working" },
  lastTurnAt: ago(60),
  standing: "working",
  rows: [{ id: `${id}-u`, at: "09:00:00", kind: "message", from: { kind: "you" }, text: title }],
  attachments: [{ kind: "branch", name: branch, slot }],
  work: { field: [{ id: "main", path, doing: "write", file }], files: [{ path: file, act: "edit" }] },
});

const own = session("jm-own", "Rotate the audit log", "armada/audit-rotation", 1, "crates/store", `${STORE}rotate.rs`);
const index = session("jm-index", "Rebuild the store index", "fix/store-index", 2, "crates/store", `${STORE}index.rs`);
const lease = session("jm-lease", "Retry the slot lease", "fix/lease-retry", 3, "crates/fleet", "crates/fleet/src/lease.rs");

const bay = (slot: number, one: Session): WorktreeSlot => ({
  manifest_id: ARMADA,
  slot,
  path: `${ROOT}/.armada/slots/slot-${slot}`,
  base: "main",
  warm: true,
  behind: 0,
  held: { state: "hosted_session", session_id: one.id, session_title: one.title! },
  branch: one.attachments[0]!.kind === "branch" ? one.attachments[0]!.name : undefined,
  since: ago(1800),
});

const pool = (): WorktreesHeld => ({ slots: [bay(1, own), bay(2, index), bay(3, lease)] });

let made = 0;
const entry = (from: Session, kind: TimelineEvent["kind"], secondsAgo: number, extra: Partial<TimelineEvent> = {}): TimelineEvent => ({
  id: 3000 + ++made,
  at: ago(secondsAgo),
  kind,
  subject: { kind: "session", id: from.id },
  title: from.title!,
  tone: "run",
  says: "",
  manifest_id: ARMADA,
  live: true,
  ...extra,
});
const edited = (from: Session, secondsAgo: number, path: string) => entry(from, "session.edited", secondsAgo, { says: "Edited 1 file", detail: path, items: [path] });
/** A message Fleet delivered from one hosted Job to another, as it writes it: `Messaged: {text}`, cut at 90 characters. */
const messaged = (from: Session, to: Session, text: string) => entry(from, "session.messaged", 0, { says: `Messaged: ${text}`.slice(0, 90), to: to.id });

function build(): Scenario {
  const base = holding("job-field-messaging", "A Job asks another Job of its repository a question on its Field, the other replies, and the arc goes stale");
  return {
    ...base,
    state: { ...base.state, repository: null },
    held: pool(),
    draft: {
      sessions: () => sessionsStore([own, index, lease], [], new Map(), []),
      events: [entry(own, "session.started", 1800, { says: "Started" }), edited(own, 40, `${STORE}rotate.rs`), edited(index, 60, `${STORE}index.rs`), edited(lease, 90, "crates/fleet/src/lease.rs")],
      eventsLater: [
        () => [messaged(own, index, "Is the store index rebuilt before rotation runs?")],
        () => [messaged(index, own, "Yes, at startup, before any rotation")],
        // Past the time a message stays fresh.
        () => ({ passes: 150 }),
      ],
      areas: mockAreas,
    },
  };
}

export const s317JobFieldMessaging: Scenario = build();
