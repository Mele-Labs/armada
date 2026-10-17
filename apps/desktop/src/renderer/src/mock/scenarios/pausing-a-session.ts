// Pausing a Session from Flight control and from its row. Pause stops the running turn, commits the work to the branch and gives the slot
// back; the Session stays listed, marked paused. Resume, or a message, takes a slot again. The claims are in `walks/pausing-a-session.ts`.

import { featureRunning } from "@armada/jobs/fake";
import type { Session } from "@armada/screens/src/draft/sessions";

import { asRow, holding, servedFrom } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

const migrate = asRow(featureRunning(), 83, "migrate", "Order the store migrations");
const OWNER = migrate.manifests[0]!.id;
const ROOT = servedFrom(migrate.manifests[0]!).root;

const RUNNING: Session = {
  id: "7c3d9a10-2b45-4e6f-8a91-0d1e2f3a4b5c",
  title: "Pin the store clock",
  manifestId: OWNER,
  cwd: ROOT,
  turn: { state: "working" },
  standing: "working",
  rows: [],
  attachments: [
    { kind: "slot", slot: 4 },
    { kind: "branch", name: "fix/pin-store-clock", slot: 4 },
  ],
};

function build(): Scenario {
  const board = [{ ...migrate, job: { ...migrate.job, owner_manifest_id: OWNER } }];
  const base = holding("pausing-a-session", "The flight deck with one working Session holding a slot and a branch", board);
  const none = { id: "x", number: 0, title: "", branch: "", slot: 0 };
  return {
    ...base,
    state: { ...base.state, repository: null, jobs: board.map((one) => one.job) },
    draft: {
      sessions: (control) => {
        const store = sessionsStore([none, none], none, [], control, [], [RUNNING]);
        let from: readonly Session[] | undefined;
        let only: readonly Session[] = [];
        return {
          ...store,
          get: () => {
            const all = store.get();
            if (all !== from) {
              from = all;
              only = all.filter((one) => one.id === RUNNING.id);
            }
            return only;
          },
          // The slot goes back with the work committed; the Session keeps its branch.
          pause: async (id) => {
            store.edit(id, ({ paused: _, ...one }) => ({ ...one, paused: true, turn: { state: "idle" }, standing: "idle", attachments: one.attachments.filter((it) => it.kind !== "slot") }));
            return true;
          },
          resume: async (id) => {
            store.edit(id, ({ paused: _, ...one }) => ({ ...one, turn: { state: "working" }, standing: "working", attachments: [{ kind: "slot", slot: 4 }, ...one.attachments] }));
            return true;
          },
        };
      },
    },
  };
}

export const s264PausingASession: Scenario = build();
