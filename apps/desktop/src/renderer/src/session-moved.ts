// What a move left on a Session, read from the ledger's fork pair, and what Fleet's refusal of one says.
// No React, so a node test reaches it.

import type { SessionThreadRow } from "@armada/components";
import type { Outcome, RepositorySummary } from "@armada/protocol";
import type { Session, SessionAttachment } from "@armada/screens/src/draft/sessions";
import { manifestLabel } from "@armada/shell";

/** Said on each repository while a terminal is still listening: Fleet cannot move the person's terminal. */
export const IN_A_TERMINAL = "Runs in a terminal that is still listening";

/** Fleet's refusals of a move, in the words the toast carries. Any other refusal is said in Fleet's own. */
const REFUSED: Readonly<Record<string, string>> = {
  "fleet.session_move_same": "Already in that repository",
  "fleet.session_move_listening": IN_A_TERMINAL,
  "fleet.no_such_session": "That Job is gone",
  "fleet.no_such_manifest": "That repository is not served",
};

/** The toast for a refused move, or `undefined` where Fleet's own words should be said. */
export function moveRefusalWords(outcome: Outcome): string | undefined {
  return outcome.ok || outcome.why !== "refused" ? undefined : REFUSED[outcome.error.code];
}

type Forked<K extends "forked_to" | "forked_from"> = Extract<SessionAttachment, { kind: K }>;

/** The Session this one was moved to, from the ledger: a fork that names the repository it went to. */
export const movedTo = (session: Session): Forked<"forked_to"> | undefined =>
  session.attachments.find((one): one is Forked<"forked_to"> => one.kind === "forked_to" && one.moved !== undefined);

/** The Session this one was moved from, from the ledger: a fork that names the repository it came from. */
export const movedFrom = (session: Session): Forked<"forked_from"> | undefined =>
  session.attachments.find((one): one is Forked<"forked_from"> => one.kind === "forked_from" && one.moved !== undefined);

/** The row that ends the old Session's thread and the one that starts the new one's, each naming the other repository. */
export function movedRowsOf(session: Session, repositories: readonly RepositorySummary[]): { head: SessionThreadRow[]; tail: SessionThreadRow[] } {
  const to = movedTo(session);
  const from = movedFrom(session);
  return {
    head: from === undefined ? [] : [{ id: `moved-from-${from.id}`, at: "", kind: "moved", way: "from", session: from.id, repository: manifestLabel(from.moved!, repositories) }],
    tail: to === undefined ? [] : [{ id: `moved-to-${to.id}`, at: "", kind: "moved", way: "to", session: to.id, repository: manifestLabel(to.moved!, repositories) }],
  };
}
