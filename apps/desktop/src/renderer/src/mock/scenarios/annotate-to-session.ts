// Notes left with the annotation layer, sent to a Session. A Fleet that
// serves two live Sessions, one of them from a terminal, and the layer on with two open notes.
// The walk `annotate-to-session` plays it. The layer's own sink and Fleet are in `../annotating.ts`.

import type { RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { FakeSessionsFleet, held, hosted, terminal } from "../sessions-fleet";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

const sessions = () => [
  hosted("01SESSIONAAAAAAAAAAAAAAAAA", { title: "Fix the flaky store test", attachments: [held("slot", "3"), held("branch", "fix/flaky-store")] }),
  terminal("01TERMINALBBBBBBBBBBBBBBBB", { title: "Release notes script" }),
];

function build(): Scenario {
  const board = onBoard({
    repositories: [ARMADA],
    picked: ARMADA.root,
  });
  const served = new FakeSessionsFleet(sessions()).scenario(board);
  return {
    ...served,
    name: "annotate-to-session",
    says: "The annotation layer on, its notes sent to a new Session or to one that is live",
    // A Fleet of its own for each window, so a walk told twice starts from the same two Sessions.
    behaves: (handle) => {
      const own = new FakeSessionsFleet(sessions()).scenario(board).behaves!(handle);
      let refused = false;
      return {
        ...own,
        // A Session Fleet starts takes its title from the first message, so the fake names it here; and a
        // thread is held before the message lands in it, which a window opening the Session does later.
        // The first start is refused with no message, as a Fleet that refused the owner's did.
        startSession: async (title) => {
          if (!refused) {
            refused = true;
            return { ok: false, outcome: { ok: false, why: "refused", error: { code: "fleet.refused", message: "", run_id: "", fields: {}, chain: [] } } };
          }
          const started = await own.startSession!(title);
          if (started.ok && title !== undefined) await own.renameSession!({ session_id: started.value.id, title });
          return started;
        },
        stageAttachment: async (_bytes, filename) => ({ path: `/tmp/${filename}` }),
        sendSessionMessage: async (send) => {
          await own.watchSession!(send.session_id);
          return own.sendSessionMessage!(send);
        },
        // A pinned note is a `send_session_note`, and the thread keeps only what the owner said of it.
        sendSessionNote: async (send) => {
          await own.watchSession!(send.session_id);
          const sent = await own.sendSessionMessage!({ session_id: send.session_id, text: send.said });
          return sent.ok ? { ok: true, value: { session: sent.value, candidates: [] } } : sent;
        },
      };
    },
  };
}

export const s202AnnotateToSession: Scenario = build();
