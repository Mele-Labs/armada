// Main going red from a Session's pull request, and the owner sending the work back to that
// Session. Drawn ahead of Fleet in `main-red-hub.ts`; the walk `mainRedSession` plays it.

import { repository } from "@armada/screens/src/fixtures/build/base";

import { holding } from "../holding";
import { FIXTURES, sessionRed, writingMainsLogs } from "../main-red-hub";
import type { Scenario } from "../moment";

function goesRed(): Scenario {
  const base = holding("main-red-session", "Main green, then red from a Session's pull request, sent back to that Session", FIXTURES);
  const { first, later } = sessionRed(repository().root);
  // Sending to the Session is Fleet's `/sessions/message`, which takes it.
  return {
    ...base,
    state: { ...base.state, ...first },
    later,
    behaves: (fleet) => ({ ...writingMainsLogs(fleet), sendSessionMessage: async () => ({ ok: true, value: {} as never }) }),
  };
}

export const s234MainRedSession: Scenario = goesRed();
