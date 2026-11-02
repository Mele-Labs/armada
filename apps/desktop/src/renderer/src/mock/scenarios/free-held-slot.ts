// A repository with three slots, held by Jobs that went quiet 288, 98 and 93 hours ago, and two Jobs whose first write finds none free.
// Each waiting Job's call keeps Wait for a slot and Add a slot, and lists the three held slots, the quietest first, each with a Free. The
// middle holder has uncommitted work, so its Free is off. Free gives that slot to the Job the call is on: its call goes, it moves on, and
// the holder stays in the list with no slot, leasing a new one when it next writes. The other waiting Job's call drops that row.
// Create is slow as a real one is: the call carries `creating_since` and the Job goes on once the slot is made, a minute on or when the walk's
// `later` moves time. The first Job's refuses, for want of disk; the second's goes through.
// The claims are in `walks/free-held-slot.ts`. The call is the wire's: both choices, and Create is the Add choice answered.

import { repository } from "@armada/screens/src/fixtures/repository";
import type { SlotHolder, WaitingItem, WorktreeSlot, WorktreesHeld } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";

import { ADD_CHOICE } from "../../held-slots";
import type { Scenario } from "../moment";
import { alsoOnTimePassing } from "../time-passes";
import type { BayControl, SessionsStore } from "../sessions/script";
import { s252CockpitShipFlightdeck as ship } from "./cockpit-ship";

const ARMADA = repository().manifest!.id;
const ITEM = "slot-choice";
const hoursAgo = (hours: number) => new Date(Date.now() - hours * 3_600_000).toISOString();
const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();
const clock = () => new Date().toTimeString().slice(0, 8);
/** How long a Create takes where no walk moves time on. */
const CREATE_TAKES_MS = 60_000;
/** The Job whose Create runs out of disk. */
const REFUSED = "n1";

/** A Job that holds a slot and has gone quiet. `keeps` is why its slot cannot be freed: absent, its branch is committed. */
type Holder = {
  session: Session;
  slot: number;
  hours: number;
  keeps?: SlotHolder["keeps"];
};

const holder = (
  id: string,
  title: string,
  slot: number,
  hours: number,
  keeps?: SlotHolder["keeps"],
): Holder => ({
  slot,
  hours,
  ...(keeps === undefined
    ? {}
    : {
        keeps,
      }),
  session: {
    id,
    title,
    manifestId: ARMADA,
    turn: {
      state: "idle",
    },
    lastTurnAt: hoursAgo(hours),
    standing: "idle",
    rows: [
      {
        id: `${id}-u`,
        at: "09:00:00",
        kind: "message",
        from: {
          kind: "you",
        },
        text: title,
      },
    ],
    attachments: [
      {
        kind: "slot",
        slot,
      },
      {
        kind: "branch",
        name: `session/${id}`,
        slot,
      },
    ],
  },
});

const HOLDERS: readonly Holder[] = [
  holder("h1", "Move the run ledger", 1, 288),
  holder("h2", "Rework the logbook filters", 2, 98, "uncommitted"),
  holder("h3", "Pocket pairing flow", 3, 93),
];

const holders = (): SlotHolder[] =>
  HOLDERS.map((one) => ({
    slot: one.slot,
    job: {
      id: one.session.id,
      title: one.session.title!,
    },
    quiet_since: hoursAgo(one.hours),
    ...(one.keeps === undefined
      ? {}
      : {
          keeps: one.keeps,
        }),
  }));

const callFor = (): WaitingItem => ({
  id: ITEM,
  text: "Every slot is in use",
  since: ago(1),
  source: "slot_choice",
  options: [
    {
      label: "Wait for a slot",
    },
    {
      label: "Add a slot",
    },
  ],
  act: {
    kind: "answer",
    target: ITEM,
  },
  holders: holders(),
});

/** At work, and held on its first write: the pool has no slot for it. */
const waiter = (id: string, title: string, edit: string): Session => ({
  id,
  title,
  manifestId: ARMADA,
  turn: {
    state: "working",
  },
  lastTurnAt: ago(1),
  standing: "working",
  rows: [
    {
      id: `${id}-u`,
      at: "14:18:00",
      kind: "message",
      from: {
        kind: "you",
      },
      text: title,
    },
    {
      id: `${id}-t0`,
      at: "14:19:02",
      kind: "tool",
      text: `Edit ${edit}`,
    },
  ],
  attachments: [],
  waitingFor: [
    callFor(),
  ],
});

const EDITS: Record<string, string> = {
  n1: "apps/desktop/src/renderer/src/cockpit/Timeline.tsx",
  n2: "apps/desktop/src/renderer/src/lessons/RetroSheet.tsx",
};
const WAITERS = [
  waiter("n1", "Chart the timeline", EDITS.n1!),
  waiter("n2", "Wire the retro sheet", EDITS.n2!),
];

/** The repository's pool is the three held slots; the ship's other repository keeps its own. */
function full(): WorktreesHeld {
  const held = ship.held!;
  const slots = HOLDERS.map(
    (one): WorktreeSlot => ({
      manifest_id: ARMADA,
      slot: one.slot,
      path: `/Users/user/armada/.armada/slots/slot-${one.slot}`,
      base: "main",
      warm: true,
      behind: 0,
      held: {
        state: "hosted_session",
        session_id: one.session.id,
        session_title: one.session.title!,
      },
      branch: `session/${one.session.id}`,
      since: hoursAgo(one.hours + 1),
    }),
  );
  return {
    ...held,
    slots: [
      ...slots,
      ...(held.slots ?? []).filter((one) => one.manifest_id !== ARMADA),
    ],
  };
}

function sessions(control: BayControl): SessionsStore {
  const store = ship.draft!.sessions!(control, [
    ...WAITERS,
    ...HOLDERS.map((one) => one.session),
  ]);
  /** The Job takes `slot`: the bay is its, it has the chips, its call is gone and it goes on with its edit. */
  const moveIn = (id: string, slot: number) => {
    const held = control.pool?.get();
    const title = WAITERS.find((one) => one.id === id)?.title;
    if (held === undefined || title === undefined) return;
    const bay: WorktreeSlot = {
      manifest_id: ARMADA,
      slot,
      path: `/Users/user/armada/.armada/slots/slot-${slot}`,
      base: "main",
      warm: true,
      behind: 0,
      held: {
        state: "hosted_session",
        session_id: id,
        session_title: title,
      },
      branch: `session/${id}`,
      since: new Date().toISOString(),
    };
    const rest = (held.slots ?? []).filter(
      (one) => !(one.manifest_id === ARMADA && one.slot === slot),
    );
    control.pool?.set({
      ...held,
      slots: [
        ...rest,
        bay,
      ].sort((a, b) => a.slot - b.slot),
    });
    store.edit(id, (one) => ({
      ...one,
      attachments: [
        ...one.attachments,
        {
          kind: "slot",
          slot,
        },
        {
          kind: "branch",
          name: `session/${id}`,
          slot,
        },
      ],
      rows: [
        ...one.rows,
        {
          id: `${id}-t${one.rows.length}`,
          at: clock(),
          kind: "tool",
          text: `Edit ${EDITS[id]}`,
        },
      ],
      waitingFor: (one.waitingFor ?? []).filter((it) => it.id !== ITEM),
    }));
  };
  const freed = new Set<number>();
  /** The Jobs a Free has served. */
  const answered = new Set<string>();
  const editCall = (id: string, change: (call: WaitingItem) => WaitingItem) =>
    store.edit(id, (one) => ({
      ...one,
      waitingFor: (one.waitingFor ?? []).map((it) => (it.id === ITEM ? change(it) : it)),
    }));
  const nextSlot = () =>
    Math.max(
      0,
      ...(control.pool?.get()?.slots ?? [])
        .filter((one) => one.manifest_id === ARMADA)
        .map((one) => one.slot),
    ) + 1;
  /** One more bay, nobody's: a Job that took a freed slot meanwhile has no call left for this one to answer. */
  const spareBay = (slot: number) => {
    const held = control.pool?.get();
    if (held === undefined) return;
    const bay: WorktreeSlot = {
      manifest_id: ARMADA,
      slot,
      path: `/Users/user/armada/.armada/slots/slot-${slot}`,
      base: "main",
      warm: true,
      behind: 0,
      held: {
        state: "free",
      },
      since: new Date().toISOString(),
    };
    control.pool?.set({
      ...held,
      slots: [
        ...(held.slots ?? []),
        bay,
      ].sort((a, b) => a.slot - b.slot),
    });
  };
  /** Creates Fleet is at, each finished once, by its timer or by the walk's `later`. */
  const making = new Map<string, () => void>();
  const finish = (id: string) => {
    if (making.delete(id)) {
      if (id === REFUSED)
        editCall(id, ({ creating_since: _, ...call }) => ({
          ...call,
          create_refused: "no space left on the disk",
        }));
      else if (answered.has(id)) spareBay(nextSlot());
      else moveIn(id, nextSlot());
    }
  };
  return {
    ...store,
    // Wait for a slot (0) takes the call off and leaves the Job queued. Add a slot (1) is Create: Fleet starts on one more bay, the call
    // stays, marked, and the Job takes the bay once it is made. A second Create while one is out changes nothing.
    answerWaiting: (id, itemId, given) => {
      if (itemId !== ITEM) return store.answerWaiting?.(id, itemId, given);
      if (given.choice === ADD_CHOICE) {
        if (making.has(id)) return;
        editCall(id, ({ create_refused: _, ...call }) => ({
          ...call,
          creating_since: new Date().toISOString(),
        }));
        const done = () => finish(id);
        making.set(id, done);
        setTimeout(done, CREATE_TAKES_MS);
        alsoOnTimePassing(() =>
          [
            ...making.values(),
          ].forEach((one) => one()),
        );
        return;
      }
      store.edit(id, (one) => ({
        ...one,
        waitingFor: (one.waitingFor ?? []).filter((it) => it.id !== ITEM),
      }));
    },
    // The slot goes to the Job the call is on. Its holder keeps its place in the list and loses the slot and its chip; the other waiting Job's call drops the row.
    freeSlot: (id, itemId, slot) => {
      if (itemId !== ITEM || freed.has(slot)) return;
      freed.add(slot);
      answered.add(id);
      const from = HOLDERS.find((one) => one.slot === slot);
      if (from !== undefined)
        store.edit(from.session.id, (one) => ({
          ...one,
          attachments: one.attachments.filter((it) => !("slot" in it && it.slot === slot)),
        }));
      moveIn(id, slot);
      for (const other of WAITERS.filter((one) => one.id !== id)) {
        store.edit(other.id, (one) => ({
          ...one,
          waitingFor: (one.waitingFor ?? []).map((it) =>
            it.id === ITEM
              ? {
                  ...it,
                  holders: it.holders?.filter((row) => row.slot !== slot),
                }
              : it,
          ),
        }));
      }
    },
  };
}

export const s299FreeHeldSlot: Scenario = {
  ...ship,
  name: "free-held-slot",
  says: "A Job's first write finds every slot held: its call lists who holds each, quietest first, with a Free on each unless the holder is running or has uncommitted work",
  held: full(),
  draft: {
    ...ship.draft,
    sessions,
  },
};
