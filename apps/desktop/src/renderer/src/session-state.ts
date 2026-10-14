// What state a Session is in, as one answer for every surface that draws it: the Sessions list and its
// head, the Cockpit's tiles and calls, and the timeline's lanes. **Read from what Fleet derived**
// (`Session.standing`, from the turn, the thread and `done_since`) and never from an event's flag or
// its age, and never from whether the agent called a tool. Pure.

import { limitHeldSaid } from "@armada/components";
import type { SessionState } from "@armada/components";
import { attachmentsOf, isBlank, owesOf, standingOf } from "@armada/screens/src/draft/sessions";
import type { Session } from "@armada/screens/src/draft/sessions";

export function stateOf(session: Session): { state: SessionState; said: string } {
  if (session.dead === "ended") return { state: "ended", said: "Ended" };
  if (session.dead === "quiet") return { state: "quiet", said: "Not reachable" };
  if (session.limitedUntil !== undefined) return { state: "limited", said: limitHeldSaid(session.limitedUntil) };
  // A turn held on a question or a permission is waiting on him, though the turn is open.
  if (session.asked !== undefined) return { state: "waiting", said: "Waiting on you" };
  const standing = standingOf(session);
  if (standing === "stuck") return { state: "stuck", said: "Stuck" };
  if (session.turn.state === "working") {
    const by = session.turn.wokenBy;
    return { state: "working", said: by === undefined ? "Working" : `Woken by ${by.title}` };
  }
  if (isBlank(session)) return { state: "blank", said: "Blank: no slot, no branch" };
  if (owesOf(session).length > 0) return { state: "waiting", said: "Waiting on you" };
  if (standing === "replied") return { state: "replied", said: "Replied" };
  if (standing === "done") return { state: "done", said: "Done" };
  const failing = attachmentsOf(session, "pull_request").find((pr) => pr.state !== "merged" && pr.checks.state === "failed");
  if (failing !== undefined) return { state: "failing", said: `Checks failed on #${failing.number}` };
  return { state: "idle", said: "Idle" };
}
