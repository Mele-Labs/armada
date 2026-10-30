// What state a Session is in, as one answer for every surface that draws it: the Sessions list and its
// head, the Cockpit's tiles and calls, and the timeline's lanes. **Read from what Fleet derived**
// (`Session.standing`, from the turn, the thread and `done_since`) and never from an event's flag or
// its age, and never from whether the agent called a tool. A reply is unread until he has been in the
// Session since it ended (`cockpit/viewed.ts`), and then the Session stands as it would without one.

import { limitHeldSaid } from "@armada/components";
import type { SessionState, WaitingLine } from "@armada/components";
import { attachmentsOf, isBlank, owesOf, standingOf } from "@armada/screens/src/draft/sessions";
import type { Session } from "@armada/screens/src/draft/sessions";
import type { HeldOpenBy } from "@armada/protocol";
import { startsWhenSaid } from "@armada/screens/src/session-waits";

import { viewedAt } from "./cockpit/viewed";

/** Whether he has been in the Session since its turn ended on a reply. */
const readSince = (session: Session): boolean => session.lastTurnAt !== undefined && viewedAt(session.id) >= Date.parse(session.lastTurnAt);

/** Whether the delivery question is what he is asked, so the Needs you card opens on it without a click. */
export const holdsDelivery = (session: Session): boolean => owesOf(session).some((one) => one.source === "delivery");

/** One thing that keeps the Job open, as the words its mark's tooltip says. Fleet derives what; this only words it. */
function waitOf(one: HeldOpenBy): WaitingLine {
  if (one.kind === "watch") return { kind: "watch", said: `Held open by ${one.name}` };
  const { number, on } = one;
  if (on.kind === "queue") return { kind: "merge", said: `#${number} in the merge queue` };
  if (on.kind === "checks") return { kind: "checks", said: on.auto_merge ? `#${number} to auto-merge once Checks pass` : `Checks running on #${number}` };
  return { kind: "merge", said: `#${number} to auto-merge` };
}

/**
 * Everything a Job waits on that is not him, in the order Fleet derived it: each live watch, then each of its own pull requests in the
 * queue, set to merge, or on its Checks. Fleet sets it only once its turn has ended and nothing waits on him (`stateOf`).
 */
export function waitsOf(session: Session): WaitingLine[] {
  return (session.heldOpenBy ?? []).map(waitOf);
}

/** Parked by him: nothing is asked of him, whatever it was waiting on, until he resumes it. An ended or unreachable Job reads as that. */
export function stateOf(session: Session): { state: SessionState; said: string } {
  if (session.paused === true && session.dead === undefined) return { state: "paused", said: "Paused" };
  if (session.dead === "ended") return { state: "ended", said: "Ended" };
  if (session.dead === "quiet") return { state: "quiet", said: "Not reachable" };
  // The agent process never came up: nothing runs, and nothing but a message from him moves it.
  if (session.startFailed !== undefined) return { state: "unstarted", said: `Failed to start: ${session.startFailed.cause}` };
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
  // holds it until he answers, a sketch or a document put to him waits on his OK, and the done ask on his tasks is his to answer, whatever its turn says.
  if (owesOf(session).some((one) => one.source === "slot_choice" || one.source === "delivery" || one.source === "plan" || one.source === "task_done" || one.source === "sketch" || one.source === "document")) return { state: "waiting", said: "Waiting on you" };
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
  // A question nobody answered in time, or kept for a walk, waits on him as an item does: Fleet keeps no item for it.
  if (owesOf(session).length > 0 || session.lapsed !== undefined) return { state: "waiting", said: "Waiting on you" };
  // Fleet lists what holds it open only while its turn has ended and nothing waits on him: a watch, or its own pull request in the queue, set to merge, or on its Checks.
  const waits = waitsOf(session);
  if (waits.length > 0) return { state: "held", said: waits.map((one) => one.said).join(" · ") };
  if (standing === "replied" && !readSince(session)) return { state: "replied", said: "Unread reply" };
  if (standing === "done") return { state: "done", said: "Done" };
  const failing = attachmentsOf(session, "pull_request").find((pr) => pr.state !== "merged" && pr.checks.state === "failed");
  if (failing !== undefined) return { state: "failing", said: `Checks failed on #${failing.number}` };
  return { state: "idle", said: "Idle" };
}
