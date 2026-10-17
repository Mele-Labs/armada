// One working Session on the flight deck, its ledger holding a slot of its own, a slot and branch a piloted Job handed it, its branch,
// a pull request and a Job. Release answers as Fleet's `release_session_holds` does for the rows named: each is given back, a slot
// with the branch it has checked out, except what the piloted Job handed over, which stays and says why. The claims are in
// `walks/session-ledger-release-row.ts`.

import { featureRunning } from "@armada/screens/src/fixtures/jobs/feature-running";
import type { HeldRow, HoldsReleased, Session, SessionAttachment } from "@armada/screens/src/draft/sessions";

import { asRow, holding, servedFrom } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const migrate = asRow(featureRunning(), 83, "migrate", "Order the store migrations");
const OWNER = migrate.manifests[0]!.id;
const ROOT = servedFrom(migrate.manifests[0]!).root;

/** Fleet's own words for a row a piloted Job holds (`crates/fleet/src/session_host/handing.rs`). */
const PILOTED_WHY = "A piloted Job holds it, and the pilot's exit gives it back.";

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
    { kind: "slot", slot: 7, handed: { job: 91 } },
    { kind: "branch", name: "fix/pin-store-clock", slot: 4 },
    { kind: "pull_request", number: 1861, title: "Pin the store clock", branch: "fix/pin-store-clock", address: "https://git.example/armada/pull/1861", checks: { state: "passed" }, state: "open", auto: false },
    { kind: "job", id: migrate.job.id, number: 83, title: migrate.job.title, state: "running", branch: "armada/83-migrate" },
  ],
};

function rowOf(one: SessionAttachment): HeldRow | undefined {
  if (one.kind === "slot") return { kind: "slot", target: String(one.slot) };
  if (one.kind === "branch") return { kind: "branch", target: one.name };
  if (one.kind === "pull_request" && one.state !== "merged") return { kind: "pr", target: String(one.number) };
  if (one.kind === "job") return { kind: "job", target: one.id };
  return undefined;
}

function build(): Scenario {
  const board = [{ ...migrate, job: { ...migrate.job, owner_manifest_id: OWNER, started_at: ago(30) } }];
  const base = holding("session-ledger-release-row", "The flight deck with one working Session whose ledger holds two slots, one a piloted Job handed it, a branch, a pull request and a Job", board);
  const none = { id: "x", number: 0, title: "", branch: "", slot: 0 };
  return {
    ...base,
    state: { ...base.state, repository: null, jobs: board.map((one) => one.job) },
    draft: {
      sessions: (control) => {
        const store = sessionsStore([none, none], none, [], control, [], [HOLDER]);
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
            const stays = (one: SessionAttachment) => (one.kind === "slot" || one.kind === "branch") && one.handed !== undefined;
            const going = acted.filter((one) => !stays(one));
            const moved: HeldRow[] = going.map((one) => {
              const row = rowOf(one)!;
              if (one.kind !== "slot") return row;
              const branch = source.attachments.find((it) => it.kind === "branch" && it.slot === one.slot);
              return { ...row, saved: { branch: branch?.kind === "branch" ? branch.name : "", files: [] } };
            });
            const kept = acted.filter(stays).map((one) => ({ ...rowOf(one)!, why: PILOTED_WHY }));
            store.edit(id, (one) => ({ ...one, attachments: one.attachments.filter((it) => !going.includes(it)) }));
            const answer: HoldsReleased = { moved, kept };
            return answer;
          },
        };
      },
    },
  };
}

export const s264SessionLedgerReleaseRow: Scenario = build();
