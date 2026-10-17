// Main going red from a Session's pull request, and the owner sending the work back to that
// Session. Drawn ahead of Fleet in `main-red-hub.ts`; the walk `mainRedSession` plays it.

import { repository } from "@armada/screens/src/fixtures/repository";

import { holding } from "../holding";
import { SESSION_REGISTRY, sessionRed, writingMainsLogs } from "../main-red-hub";
import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

function goesRed(): Scenario {
  const base = holding("main-red-session", "Main green, then red from a Session's pull request, sent back to that Session");
  const { first, later } = sessionRed(repository().root);
  // Sending to the Session is Fleet's `/sessions/message`, which takes it.
  return {
    ...base,
    state: { ...base.state, ...first },
    later,
    draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, SESSION_REGISTRY) },
    behaves: (fleet) => ({ ...writingMainsLogs(fleet), sendSessionMessage: async () => ({ ok: true, value: {} as never }) }),
  };
}

export const s234MainRedSession: Scenario = goesRed();
