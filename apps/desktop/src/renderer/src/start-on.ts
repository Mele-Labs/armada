// A Job opened on a repository, empty and ready to type in: how a first folder and a new project both end. The Job is selected on the deck's own store,
// which the Cockpit reads.

import type { SessionsDraft } from "@armada/screens/src/draft/sessions";

import { select } from "./cockpit/deck/store";
import { askComposerFocus } from "./session-focus";

/** Starts a Job on `root` and focuses its message box. The picker is left where it was: on All repositories the Job sits under its repository's heading. */
export async function startOn(draft: SessionsDraft, root: string): Promise<void> {
  const id = await draft.start(undefined, root);
  if (id !== undefined) {
    askComposerFocus(id);
    select({ kind: "session", key: `session:${id}` });
  }
}
