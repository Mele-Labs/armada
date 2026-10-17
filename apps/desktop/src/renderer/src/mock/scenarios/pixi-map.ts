// The Cockpit map with work going on, for the walk `pixi-map`: the ship at rest, and over it one Session writing now, one reading and a Job
// running a Check. Three Sessions name the files they touch, and every room holds a pill for each of its files; those three lit on theirs (`full-rooms`). The
// walk plays it on the CSS map and on the canvas one, so the two can be set side by side.

import type { TimelineEvent } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";

import { mockRooms } from "../cockpit-areas";
import type { Scenario } from "../moment";
import { s252CockpitShipFlightdeck as ship } from "./cockpit-ship";

type Work = NonNullable<Session["work"]>;

/** What each live Session names, by its id: the files its Field draws, and where its own drone is over one. A file with no act is one it has read. */
const WORK: Record<string, Pick<Work, "files" | "field">> = {
  // Pin the store clock: writing now.
  r1: {
    files: [{ path: "crates/store/tests/clock.rs", act: "write" }, { path: "crates/store/tests/pool.rs", act: "edit" }, { path: "crates/store/src/clock.rs" }],
    field: [{ id: "main", path: "crates/store", doing: "write", file: "crates/store/tests/clock.rs" }],
  },
  // Migration notes: reading.
  w1: { files: [{ path: "crates/store/migrations/0044.sql" }, { path: "crates/store/migrations/0043.sql" }] },
  // Gate policy on every run: edits the gate. Where it and the clock Session both name a file, the stronger act stands.
  p1: { files: [{ path: "crates/gate/src/policy.rs", act: "edit" }, { path: "crates/fleet/src/gate.rs", act: "edit" }, { path: "crates/store/tests/pool.rs", act: "write" }] },
};

const held = ship.draft?.events ?? [];
const writer = held.find((event) => event.subject.kind === "session" && event.subject.id === "r1");

/** A burst of edits a moment ago, so the Session that made them is writing as the map draws it. */
const WRITING: TimelineEvent[] =
  writer === undefined
    ? []
    : [{ id: 9001, at: new Date().toISOString(), kind: "session.edited", subject: writer.subject, title: writer.title, tone: "run", says: "Edited 1 file", manifest_id: writer.manifest_id, detail: "crates/store/tests/clock.rs", items: ["crates/store/tests/clock.rs"], live: true }];

function build(): Scenario {
  const draft = ship.draft!;
  return {
    ...ship,
    name: "pixi-map",
    says: "The Cockpit map at rest with one Session writing, one reading, a Job running a Check, and a pill for every file in each room, lit for the files three Sessions touch",
    draft: {
      ...draft,
      areas: mockRooms,
      events: [...held, ...WRITING],
      sessions: (control) => {
        const store = draft.sessions!(control);
        let from: readonly Session[] | undefined;
        let shown: readonly Session[] = [];
        return {
          ...store,
          get: () => {
            const all = store.get();
            if (all !== from) {
              from = all;
              shown = all.map((one) => (WORK[one.id] === undefined ? one : { ...one, work: { ...one.work, ...WORK[one.id] } }));
            }
            return shown;
          },
        };
      },
    },
  };
}

export const pixiMapShip: Scenario = build();
