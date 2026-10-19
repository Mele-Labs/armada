// Two Sessions of one repository on the flight deck. The first is working and holds two slots, a branch and a pull request;
// the second holds a slot and a branch of its own. Release answers as Fleet's `release_session_holds` does, for the rows ticked:
// a hand moves each to the Session picked, and a release gives each back, committing a slot's files first, except the slot whose checkout is on no
// branch, which stays and says why. The claims are in `walks/cockpit-release-holds.ts`.

import type { HeldRow, HoldsReleased, Session, SessionAttachment } from "@armada/screens/src/draft/sessions";
import { repository } from "@armada/screens/src/fixtures/repository";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const OWNER = repository().manifest!.id;
const ROOT = repository().root;

/** The slot whose checkout is on no branch: a release cannot commit its files, so it stays. */
const DETACHED = 6;
const DETACHED_WHY = "The checkout at slot-6 is on no branch, so its files were not committed and the slot stays held.";

const HOLDER: Session = {
  id: "7c3d9a10-2b45-4e6f-8a91-0d1e2f3a4b5c",
  title: "Pin the store clock",
  manifestId: OWNER,
  cwd: ROOT,
  turn: { state: "working" },
  standing: "working",
  rows: [],
  attachments: [
    { kind: "slot", slot: 4 },
    { kind: "slot", slot: DETACHED },
    { kind: "branch", name: "fix/pin-store-clock", slot: 4 },
    { kind: "pull_request", number: 1861, title: "Pin the store clock", branch: "fix/pin-store-clock", address: "https://git.example/armada/pull/1861", checks: { state: "passed" }, state: "open", auto: false },
  ],
};

const TAKER: Session = {
  id: "9e8f7a6b-5c4d-4e3f-9a2b-1c0d9e8f7a6b",
  title: "Review the store tests",
  manifestId: OWNER,
  cwd: ROOT,
  turn: { state: "idle" },
  standing: "idle",
  lastTurnAt: ago(12),
  rows: [],
  attachments: [
    { kind: "slot", slot: 2 },
    { kind: "branch", name: "review/store-tests", slot: 2 },
  ],
};

/** The ledger row a draft attachment is, as Fleet names it in its answer. Artifacts and links are not acted on. */
function rowOf(one: SessionAttachment): HeldRow | undefined {
  if (one.kind === "slot") return { kind: "slot", target: String(one.slot) };
  if (one.kind === "branch") return { kind: "branch", target: one.name };
  if (one.kind === "pull_request" && one.state !== "merged") return { kind: "pr", target: String(one.number) };
  return undefined;
}

function build(): Scenario {
  const base = holding("cockpit-release-holds", "The flight deck with two Sessions of one repository, the first working and holding two slots, a branch and a pull request");
  return {
    ...base,
    state: { ...base.state, repository: null },
    draft: {
      sessions: () => {
        const store = sessionsStore([HOLDER, TAKER]);
        const own = new Set([HOLDER.id, TAKER.id]);
        let from: readonly Session[] | undefined;
        let only: readonly Session[] = [];
        return {
          ...store,
          get: () => {
            const all = store.get();
            if (all !== from) {
              from = all;
              only = all.filter((one) => own.has(one.id));
            }
            return only;
          },
          releaseHolds: async (id, to, rows) => {
            const source = store.get().find((one) => one.id === id);
            if (source === undefined) return undefined;
            const picked = (row: { kind: string; target: string }) => rows === undefined || rows.some((it) => it.kind === row.kind && it.target === row.target);
            const slots = source.attachments.filter((one) => one.kind === "slot" && picked(rowOf(one)!)).map((one) => (one.kind === "slot" ? one.slot : 0));
            // A slot takes the branch it has checked out with it, as Fleet's does.
            const acted = source.attachments.filter((one) => {
              const row = rowOf(one);
              return row !== undefined && (picked(row) || (one.kind === "branch" && slots.includes(one.slot)));
            });
            // A hand moves the lease as it stands, so nothing is committed and nothing can stay.
            const stays = (one: SessionAttachment) => to === undefined && one.kind === "slot" && one.slot === DETACHED;
            const going = acted.filter((one) => !stays(one));
            const moved: HeldRow[] = going.map((one) => {
              const row = rowOf(one)!;
              if (to !== undefined || one.kind !== "slot") return row;
              const branch = source.attachments.find((it) => it.kind === "branch" && it.slot === one.slot);
              const on = branch?.kind === "branch" ? branch.name : "";
              // Slot 4 holds the clock tests the agent had not committed; any other slot is clean and only freed.
              return one.slot === 4 ? { ...row, saved: { branch: on, commit: "a41c9e2", files: ["crates/store/tests/clock.rs", "crates/store/tests/pool.rs"] } } : { ...row, saved: { branch: on, files: [] } };
            });
            const kept = acted.filter(stays).map((one) => ({ ...rowOf(one)!, why: DETACHED_WHY }));
            // Only a slot moved from under the agent stops its turn.
            const stopped = slots.length > 0 ? { turn: { state: "idle" as const }, standing: "idle" as const } : {};
            store.edit(id, (one) => ({
              ...one,
              ...stopped,
              attachments: one.attachments.filter((it) => !going.includes(it)),
            }));
            if (to !== undefined) {
              store.edit(to, (one) => ({
                ...one,
                attachments: [...one.attachments, ...going],
              }));
            }
            const answer: HoldsReleased = { ...(to === undefined ? {} : { to }), moved, kept };
            return answer;
          },
        };
      },
    },
  };
}

export const s263CockpitReleaseHolds: Scenario = build();
