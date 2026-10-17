// What the Field draws of a Session's drones: the Session's own, and only the subagents at work. A subagent that has finished has left the
// Field; the Subagents layer still lists it. A subagent at work that no file has been seen touched by is not in Fleet's touches, so the Field
// has no place for it from them: it is drawn from the ledger's own `subagent` row, at rest on the strip until an edit places it.

import type { Session, WorkDrone } from "@armada/screens/src/draft/sessions";
import { attachmentsOf } from "@armada/screens/src/draft/sessions";

/** The Session's own drone before any work has placed it. */
const OWN: WorkDrone = { id: "main", path: "", doing: "read" };

export function workingDrones(session: Session, field: readonly WorkDrone[] | undefined): readonly WorkDrone[] | undefined {
  const subagents = attachmentsOf(session, "subagent");
  const finished = new Set(subagents.flatMap((one) => (one.state === "done" ? [one.id] : [])));
  const placed = new Set((field ?? []).map((one) => one.id));
  const unplaced = subagents.flatMap((one): WorkDrone[] =>
    one.state === "running" && !placed.has(one.id) ? [{ id: one.id, agent: one.agentType ?? "Subagent", task: one.task, path: "", doing: "read" }] : [],
  );
  if (field === undefined && unplaced.length === 0) return undefined;
  return [...(field ?? [OWN]).filter((one) => !finished.has(one.id)), ...unplaced];
}

/** A drone at work: one a plot is known for, or a subagent, which is at work while the ledger holds it running. */
export const isWorking = (one: WorkDrone): boolean => one.path !== "" || one.agent !== undefined;
