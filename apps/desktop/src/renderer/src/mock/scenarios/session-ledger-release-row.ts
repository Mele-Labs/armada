// One working Session on the flight deck, its ledger holding a slot, its branch and a pull request. Release answers as Fleet's
// `release_session_holds` does for the rows named: each is given back, a slot with the branch it has checked out. The claims are in
// `walks/session-ledger-release-row.ts`.

import { repository } from "@armada/screens/src/fixtures/repository";
import type { HeldRow, HoldsReleased, Session, SessionAttachment } from "@armada/screens/src/draft/sessions";

import { connected } from "../moment";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

const OWNER = repository().manifest!.id;
const ROOT = repository().root;

const HOLDER: Session = {
  id: "5a6b7c8d-9e0f-4a1b-8c2d-3e4f5a6b7c8d",
  title: "Pin the store clock",
  manifestId: OWNER,
  cwd: ROOT,
  turn: { state: "working" },
  standing: "working",
  rows: [],
  attachments: [
    { kind: "slot", slot: 4 },
    { kind: "branch", name: "fix/pin-store-clock", slot: 4 },
    { kind: "pull_request", number: 1861, title: "Pin the store clock", branch: "fix/pin-store-clock", address: "https://git.example/armada/pull/1861", checks: { state: "passed" }, state: "open", auto: false },
  ],
};

function rowOf(one: SessionAttachment): HeldRow | undefined {
  if (one.kind === "slot") return { kind: "slot", target: String(one.slot) };
  if (one.kind === "branch") return { kind: "branch", target: one.name };
  if (one.kind === "pull_request" && one.state !== "merged") return { kind: "pr", target: String(one.number) };
  return undefined;
}

function build(): Scenario {
  return {
    name: "session-ledger-release-row",
    says: "The flight deck with one working Session whose ledger holds a slot, a branch and a pull request",
    state: { ...connected([], [repository()]), repository: null },
    draft: {
      sessions: () => {
        const store = sessionsStore([HOLDER]);
        let from: readonly Session[] | undefined;
        let only: readonly Session[] = [];
        return {
          ...store,
          get: () => {
            const all = store.get();
            if (all !== from) {
              from = all;
              only = all.filter((one) => one.id === HOLDER.id);
            }
            return only;
          },
          releaseHolds: async (id, _to, rows) => {
            const source = store.get().find((one) => one.id === id);
            if (source === undefined) return undefined;
            const picked = (row: HeldRow) => rows === undefined || rows.some((it) => it.kind === row.kind && it.target === row.target);
            const slots = source.attachments.flatMap((one) => (one.kind === "slot" && picked(rowOf(one)!) ? [one.slot] : []));
            const acted = source.attachments.filter((one) => {
              const row = rowOf(one);
              return row !== undefined && (picked(row) || (one.kind === "branch" && slots.includes(one.slot)));
            });
            const moved: HeldRow[] = acted.map((one) => {
              const row = rowOf(one)!;
              if (one.kind !== "slot") return row;
              const branch = source.attachments.find((it) => it.kind === "branch" && it.slot === one.slot);
              return { ...row, saved: { branch: branch?.kind === "branch" ? branch.name : "", files: [] } };
            });
            store.edit(id, (one) => ({ ...one, attachments: one.attachments.filter((it) => !acted.includes(it)) }));
            const answer: HoldsReleased = { moved, kept: [] };
            return answer;
          },
        };
      },
    },
  };
}

export const s264SessionLedgerReleaseRow: Scenario = build();
