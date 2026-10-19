// Every slot of the pool is taken when a Session's first write wants one. Fleet puts a call on the Session: wait for a slot or add
// one. The call shows on the Cockpit and in the Session's Needs you band. Add a slot makes a bay and the Session moves into it; Wait
// leaves it queued, and it takes the next bay that is free or made. The Bays panel adds a bay of its own. Over the cockpit ship,
// its one free bay now held. The claims are in `walks/slot-full-ask.ts`.

import { repository } from "@armada/screens/src/fixtures/repository";
import type { WaitingItem, WorktreeSlot, WorktreesHeld } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import type { BayControl, SessionsStore } from "../sessions/script";
import { s252CockpitShipFlightdeck as ship } from "./cockpit-ship";

const ARMADA = repository().manifest!.id;
const ITEM = "slot-choice";
const BRANCH = "session/chart-the-timeline";
const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();
const clock = () => new Date().toTimeString().slice(0, 8);
let rows = 0;
const tool = (text: string) => ({ id: `n1-t${(rows += 1)}`, at: clock(), kind: "tool", text }) as const;

const CHOICE: WaitingItem = {
  id: ITEM,
  text: "Every slot is in use",
  since: ago(1),
  source: "slot_choice",
  options: [{ label: "Wait for a slot" }, { label: "Add a slot" }],
  act: { kind: "answer", target: ITEM },
};

/** At work, and held on its first write: the pool has no slot for it. */
const WAITER: Session = {
  id: "n1",
  title: "Chart the timeline",
  manifestId: ARMADA,
  turn: { state: "working" },
  lastTurnAt: ago(1),
  standing: "working",
  rows: [
    { id: "n1-u", at: "14:18:00", kind: "message", from: { kind: "you" }, text: "Chart the activity timeline by repository." },
    { id: "n1-t0", at: "14:19:02", kind: "tool", text: "Edit apps/desktop/src/renderer/src/cockpit/Timeline.tsx" },
  ],
  attachments: [],
  waitingFor: [CHOICE],
};

/** The ship's pool with its one free bay held, so none is left. */
function full(): WorktreesHeld {
  const held = ship.held!;
  const taken = (one: WorktreeSlot): WorktreeSlot =>
    one.manifest_id === ARMADA && one.slot === 5
      ? { ...one, held: { state: "hosted_session", session_id: "w1", session_title: "Migration notes" }, branch: "armada/migration-notes", since: ago(12), warm: true }
      : one;
  return { ...held, slots: (held.slots ?? []).map(taken) };
}

/** A bay of the repository that nothing holds and a person has not closed: free, or a slot not made yet. */
const open = (one: WorktreeSlot) => one.manifest_id === ARMADA && one.closed !== true && (one.held.state === "free" || one.held.state === "unmade");

/** The Session moves into a bay: a free one when `free`, else a new one, and it stops waiting. */
function moveIn(control: BayControl, store: SessionsStore, id: string, free: boolean): boolean {
  const held = control.pool?.get();
  if (held === undefined) return false;
  const slots = held.slots ?? [];
  const taking = free ? slots.find(open) : undefined;
  if (free && taking === undefined) return false;
  const number = taking?.slot ?? Math.max(0, ...slots.filter((one) => one.manifest_id === ARMADA).map((one) => one.slot)) + 1;
  const bay: WorktreeSlot = {
    manifest_id: ARMADA,
    slot: number,
    path: `/Users/user/armada/.armada/slots/slot-${number}`,
    base: "main",
    warm: taking?.warm ?? true,
    behind: 0,
    ...taking,
    held: { state: "hosted_session", session_id: id, session_title: WAITER.title ?? id },
    branch: BRANCH,
    since: new Date().toISOString(),
  };
  const next = taking === undefined ? [...slots, bay] : slots.map((one) => (one === taking ? bay : one));
  control.pool?.set({ ...held, slots: next.sort((a, b) => a.slot - b.slot) });
  store.edit(id, (one) => ({
    ...one,
    attachments: [...one.attachments, { kind: "slot", slot: number }, { kind: "branch", name: BRANCH, slot: number }],
    rows: [...one.rows, tool("Edit apps/desktop/src/renderer/src/cockpit/Timeline.tsx")],
    waitingFor: (one.waitingFor ?? []).filter((it) => it.id !== ITEM),
  }));
  return true;
}

function sessions(control: BayControl): SessionsStore {
  const store = ship.draft!.sessions!(control, [WAITER]);
  let queued = false;
  return {
    ...store,
    // Wait for a slot (0) leaves the Session queued; Add a slot (1) makes one and takes it. Anything else is the ship's.
    answerWaiting: (id, itemId, given) => {
      if (itemId !== ITEM) return store.answerWaiting?.(id, itemId, given);
      if (given.choice === 1) return void moveIn(control, store, id, false);
      queued = true;
      store.edit(id, (one) => ({ ...one, rows: [...one.rows, tool("Waiting for a slot")], waitingFor: (one.waitingFor ?? []).filter((it) => it.id !== ITEM) }));
    },
    // A queued Session takes the next bay that is free, or one a person added, when time passes.
    later() {
      store.later();
      if (queued && moveIn(control, store, WAITER.id, true)) queued = false;
    },
  };
}

export const s266SlotFullAsk: Scenario = {
  ...ship,
  name: "slot-full-ask",
  says: "A Session's first write finds every slot taken: its call offers waiting or adding a slot, and the Bays panel adds one of its own",
  held: full(),
  draft: { ...ship.draft, sessions },
};
