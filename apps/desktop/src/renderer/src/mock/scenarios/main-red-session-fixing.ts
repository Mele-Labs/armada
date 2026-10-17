// Main red from a Session's pull request, and that Session holding the red. The band names it in the
// Fixing row and offers nothing. Drawn ahead of Fleet in `main-red-hub.ts`; the
// walk `mainRedSessionFixing` plays it.

import { repository } from "@armada/screens/src/fixtures/repository";

import { holding } from "../holding";
import { SESSION_REGISTRY, sessionFixing, writingMainsLogs } from "../main-red-hub";
import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

function fixing(): Scenario {
  const base = holding("main-red-session-fixing", "Main red from a Session's pull request, and that Session holding the red");
  const { first, later } = sessionFixing(repository().root);
  return {
    ...base,
    state: { ...base.state, ...first },
    later,
    draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, SESSION_REGISTRY) },
    behaves: (fleet) => writingMainsLogs(fleet),
  };
}

export const s254MainRedSessionFixing: Scenario = fixing();
