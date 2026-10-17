// What the Field draws of a Session's drones: the Session's own, and only the subagents at work. A subagent that has finished has left the
// Field; the Subagents layer still lists it.

import type { Session, WorkDrone } from "@armada/screens/src/draft/sessions";
import { attachmentsOf } from "@armada/screens/src/draft/sessions";

export function workingDrones(session: Session, field: readonly WorkDrone[] | undefined): readonly WorkDrone[] | undefined {
  if (field === undefined) return undefined;
  const finished = new Set(attachmentsOf(session, "subagent").flatMap((one) => (one.state === "done" ? [one.id] : [])));
  return field.filter((one) => !finished.has(one.id));
}
