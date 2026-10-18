// What the Field draws of a Session's drones: the Session's own, and only the subagents at work. A subagent that has finished has left the
// Field; the Subagents layer still lists it. A subagent at work that no file has been seen touched by is not in Fleet's touches, so the Field
// has no place for it from them: it is drawn from the ledger's own `subagent` row, at rest on the strip until an edit places it.
// Fleet keys a hosted subagent's touches by the call that started it, and the ledger's row by the id the engine gave it: the row names the call.

import type { Session, SessionAttachment, WorkDrone } from "@armada/screens/src/draft/sessions";
import { attachmentsOf } from "@armada/screens/src/draft/sessions";

/** The Session's own drone before any work has placed it. */
const OWN: WorkDrone = { id: "main", path: "", doing: "read" };

type Subagent = Extract<SessionAttachment, { kind: "subagent" }>;

const subagentsOf = (session: Session): Subagent[] => attachmentsOf(session, "subagent");

export function workingDrones(session: Session, field: readonly WorkDrone[] | undefined): readonly WorkDrone[] | undefined {
  const subagents = subagentsOf(session);
  const row = new Map<string, Subagent>();
  for (const one of subagents) {
    if (one.call !== undefined) row.set(one.call, one);
    row.set(one.id, one);
  }
  // A drone Fleet placed is the row it names, titled by the row's type and task, and gone once the row is done.
  const placed = (field ?? []).flatMap((one): WorkDrone[] => {
    const held = one.agent === undefined ? undefined : row.get(one.id);
    if (held === undefined) return [one];
    if (held.state === "done") return [];
    return [{ ...one, id: held.id, agent: held.agentType ?? one.agent, task: held.task }];
  });
  const drawn = new Set(placed.map((one) => one.id));
  const unplaced = subagents.flatMap((one): WorkDrone[] =>
    one.state === "running" && !drawn.has(one.id) ? [{ id: one.id, agent: one.agentType ?? "Subagent", task: one.task, path: "", doing: "read" }] : [],
  );
  if (field === undefined && unplaced.length === 0) return undefined;
  return [...(field === undefined ? [OWN] : placed), ...unplaced];
}

/** A drone at work: the Session's own while its turn runs, a subagent while the ledger holds it running, and one the ledger does not hold while the turn runs. */
export function isWorking(session: Session, one: WorkDrone): boolean {
  const turn = session.turn.state === "working";
  if (one.agent === undefined) return turn && one.path !== "";
  const held = subagentsOf(session).find((row) => row.id === one.id);
  return held === undefined ? turn : held.state === "running";
}
