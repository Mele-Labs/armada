// The handoff a fresh Session opens on. **The old Session writes it**: it is sent the ask as any message,
// and its own next reply is the handoff. A Session that cannot answer in time (it ended, the send was
// refused, or its turn runs on) is given the ledger's facts instead.

import { attachmentsOf } from "@armada/screens/src/draft/sessions";
import type { Session, SessionRow, SessionsDraft } from "@armada/screens/src/draft/sessions";

/** What the old Session is asked. It is a message of the person's, so it is read in the thread. */
export const HANDOFF_ASK = "Write a short handoff for a new Job: the goal, where it stands, open pull requests and branches, and what is next.";

/** How long the old Session has to answer before the ledger's facts stand in. */
export const HANDOFF_WITHIN_MS = 90_000;

const heading = (session: Session): string => `Handoff from ${session.title ?? session.address ?? session.id}`;

/** The Session's title, then its slot, branches and open pull requests, one to a line. */
export function handoffFromLedger(session: Session): string {
  const lines = [heading(session)];
  const slots = attachmentsOf(session, "slot").map((one) => one.slot);
  const branches = attachmentsOf(session, "branch").map((one) => one.name);
  const pulls = attachmentsOf(session, "pull_request").filter((one) => one.state !== "merged");
  for (const slot of slots) lines.push(`Slot ${slot}`);
  for (const branch of branches) lines.push(`Branch ${branch}`);
  for (const pull of pulls) lines.push(`Pull request #${pull.number} ${pull.branch}`);
  return lines.join("\n");
}

const asks = (rows: readonly SessionRow[]): number => rows.filter((one) => one.kind === "message" && one.from.kind === "you" && one.text === HANDOFF_ASK).length;

/** The agent's last message after the latest ask, where one has been asked. */
function replyTo(rows: readonly SessionRow[]): string | undefined {
  let from = -1;
  rows.forEach((one, at) => {
    if (one.kind === "message" && one.from.kind === "you" && one.text === HANDOFF_ASK) from = at;
  });
  if (from < 0) return undefined;
  for (let at = rows.length - 1; at > from; at -= 1) {
    const one = rows[at];
    if (one?.kind === "message" && one.from.kind === "agent" && one.text.trim() !== "") return one.text;
  }
  return undefined;
}

/**
 * Asks the Session for its handoff and resolves to its reply once the turn is over, or to nothing: the
 * send was refused, the Session ended, or no reply came within `within`.
 */
export async function askForHandoff(draft: SessionsDraft, id: string, within: number = HANDOFF_WITHIN_MS): Promise<string | undefined> {
  const before = asks(draft.get().find((one) => one.id === id)?.rows ?? []);
  const sent = await draft.send(id, { text: HANDOFF_ASK, files: [], sketches: [], tags: [] });
  if (sent === false) return undefined;
  return new Promise<string | undefined>((resolve) => {
    let off = (): void => undefined;
    const finish = (text: string | undefined): void => {
      clearTimeout(timer);
      off();
      resolve(text);
    };
    const check = (): void => {
      const session = draft.get().find((one) => one.id === id);
      if (session === undefined || session.dead !== undefined) return finish(undefined);
      // The ask is in the thread, and the turn it started is over.
      if (session.turn.state !== "idle" || asks(session.rows) <= before) return;
      const reply = replyTo(session.rows);
      if (reply !== undefined) finish(reply);
    };
    const timer = setTimeout(() => finish(undefined), within);
    off = draft.subscribe(check);
    check();
  });
}

/** The first message of the fresh Session: the old one's own handoff under a heading, or the ledger's facts. */
export async function handoffOf(draft: SessionsDraft, session: Session, within: number = HANDOFF_WITHIN_MS): Promise<string> {
  const reply = await askForHandoff(draft, session.id, within);
  const now = draft.get().find((one) => one.id === session.id) ?? session;
  return reply === undefined ? handoffFromLedger(now) : `${heading(now)}\n\n${reply}`;
}
