// What state a Session is in, as one answer for every surface that draws it: the Sessions list and its
// head, the Cockpit's tiles and calls, and the timeline's lanes. **Read from what Fleet derived**
// (`Session.standing`, from the turn, the thread and `done_since`) and never from an event's flag or
// its age, and never from whether the agent called a tool. A reply is unread until he has been in the
// Session since it ended (`cockpit/viewed.ts`), and then the Session stands as it would without one.

import { limitHeldSaid } from "@armada/components";
import type { SessionState } from "@armada/components";
import { attachmentsOf, isBlank, owesOf, standingOf } from "@armada/screens/src/draft/sessions";
import type { Session } from "@armada/screens/src/draft/sessions";
import { startsWhenSaid } from "@armada/screens/src/session-waits";

import { viewedAt } from "./cockpit/viewed";

/** Whether he has been in the Session since its turn ended on a reply. */
const readSince = (session: Session): boolean => session.lastTurnAt !== undefined && viewedAt(session.id) >= Date.parse(session.lastTurnAt);

/** Whether the delivery question is what he is asked, so the Needs you card opens on it without a click. */
export const holdsDelivery = (session: Session): boolean => owesOf(session).some((one) => one.source === "delivery");

/** Parked by him: nothing is asked of him, whatever it was waiting on, until he resumes it. An ended or unreachable Job reads as that. */
export function stateOf(session: Session): { state: SessionState; said: string } {
  if (session.paused === true && session.dead === undefined) return { state: "paused", said: "Paused" };
  if (session.dead === "ended") return { state: "ended", said: "Ended" };
  if (session.dead === "quiet") return { state: "quiet", said: "Not reachable" };
  // Dispatched after another Session: nothing runs, and nothing is asked of him, until its condition is met.
  if (session.waits !== undefined) return { state: "after", said: startsWhenSaid(session.waits) };
  // The proposer is reading the request: the Session starts on its answer.
  if (session.work?.proposal?.reading === true) return { state: "working", said: "Reading the request" };
  // Read and held on its start card: nothing has moved, and nothing is asked of him but Start.
  if (session.start !== undefined) return { state: "draft", said: "Not started" };
  if (session.limitedUntil !== undefined) return { state: "limited", said: limitHeldSaid(session.limitedUntil) };
  // A turn held on a question or a permission is waiting on him, though the turn is open.
  if (session.asked !== undefined) return { state: "waiting", said: "Waiting on you" };
  // A full pool holds the agent's first write, the delivery question holds the agent at its start, a plan put to him
  // holds it until he answers, and the done ask on his tasks is his to answer, whatever its turn says.
  if (owesOf(session).some((one) => one.source === "slot_choice" || one.source === "delivery" || one.source === "plan" || one.source === "task_done")) return { state: "waiting", said: "Waiting on you" };
  const standing = standingOf(session);
  if (standing === "stuck") return { state: "stuck", said: "Stuck" };
  if (session.turn.state === "working") {
    const by = session.turn.wokenBy;
    return { state: "working", said: by === undefined ? "Working" : `Woken by ${by.title}` };
  }
  // Fleet's word covers what a turn of its own does not: a subagent at work, a terminal's turn.
  if (standing === "working") {
    const subagent = attachmentsOf(session, "subagent").some((one) => one.state === "running");
    return { state: "working", said: subagent ? "Subagent working" : "Working" };
  }
  if (isBlank(session)) return { state: "blank", said: "Blank: no slot, no branch" };
  if (owesOf(session).length > 0) return { state: "waiting", said: "Waiting on you" };
  // Fleet names the watch only while its turn has ended, nothing waits on him and a live watch is all that keeps it open.
  if (session.heldOpenBy !== undefined) return { state: "held", said: `Held open by ${session.heldOpenBy}` };
  if (standing === "replied" && !readSince(session)) return { state: "replied", said: "Unread reply" };
  if (standing === "done") return { state: "done", said: "Done" };
  const failing = attachmentsOf(session, "pull_request").find((pr) => pr.state !== "merged" && pr.checks.state === "failed");
  if (failing !== undefined) return { state: "failing", said: `Checks failed on #${failing.number}` };
  return { state: "idle", said: "Idle" };
}
