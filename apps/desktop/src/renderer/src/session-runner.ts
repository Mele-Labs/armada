// Runs a command where a Session works. **One path** for the Needs-you ledger and a call card: a clean
// exit settles the item that asked for that command, and the agent is told.

import type { CodeRunner } from "@armada/components";
import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";

/**
 * `settle` is how the item is settled once the command exits clean: given what settles it, it may
 * hold that back, as a card does to show the exit before it goes. Left out, it settles at once.
 */
export function runnerOf(draft: SessionsDraft, session: Session, settle: (now: () => void) => void = (now) => now()): CodeRunner | undefined {
  const run = draft.run;
  if (run === undefined) return undefined;
  return (command, on) =>
    run(session.id, command, (event) => {
      on(event);
      if (event.kind !== "end" || event.code !== 0) return;
      const asked = (session.waitingFor ?? []).find((one) => one.act?.kind === "run" && one.act.target.trim() === command.trim());
      if (asked !== undefined) settle(() => draft.answerWaiting?.(session.id, asked.id, { text: "Ran: exit 0" }));
    });
}
