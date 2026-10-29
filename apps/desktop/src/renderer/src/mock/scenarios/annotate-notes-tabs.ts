// The annotation layer on over eight notes on three screens, some sent to a Job and some done, for the panel's
// search, tabs, folding groups and Select all. A Fleet that starts the Job the ticked notes go to.
// The walk `annotate-notes-tabs` plays it. The notes are in `../annotating.tsx`.

import type { RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { FakeSessionsFleet, asking, held, hosted } from "../sessions-fleet";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

// The Jobs the sent notes went to: one working, one waiting on a call. The fixtures' notes name them by id.
const sessions = () => [
  hosted("01SESSIONNOTESTABSAAAAAAAA", { title: "Tidy the invoice table", attachments: [held("slot", "2"), held("branch", "fix/invoice-table")] }),
  hosted("01SESSIONSENTROWBADGEAAAAA", { title: "Row badge spacing", hosted: { turn: { state: "working" }, mode: "auto", running: true } }),
  hosted("01SESSIONSENTCOMPOSERAAAAA", { title: "Composer draft", hosted: { turn: { state: "working" }, mode: "auto", running: true, asked: asking("pnpm test --filter composer") } }),
];

function build(): Scenario {
  const board = onBoard({ repositories: [ARMADA], picked: ARMADA.root });
  return {
    ...new FakeSessionsFleet(sessions()).scenario(board),
    name: "annotate-notes-tabs",
    says: "The annotation layer's notes panel: search, tabs, folded screens, and a Job started for the ticked notes",
    // A Fleet of its own for each window, so a walk told twice starts from the same Job.
    behaves: (handle) => {
      const own = new FakeSessionsFleet(sessions()).scenario(board).behaves!(handle);
      return {
        ...own,
        // A Session Fleet starts takes its title from the first message, so the fake names it here.
        startSession: async (title) => {
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

export const annotateNotesTabs: Scenario = build();
