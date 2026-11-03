// What state a Session is in, as one answer for every surface that draws it: the Sessions list and its
// head, the Cockpit's tiles and calls, and the timeline's lanes. **Read from what Fleet derived**
// (`Session.standing`, from the turn, the thread and `done_since`) and never from an event's flag or
// its age, and never from whether the agent called a tool. A reply is unread until he has been in the
// Session since it ended (`cockpit/viewed.ts`), and then the Session stands as it would without one.

import { limitHeldSaid, timeFormat, timeSaid } from "@armada/components";
import type { SessionState, WaitingLine } from "@armada/components";
import { attachmentsOf, isBlank, owesOf, standingOf } from "@armada/screens/src/draft/sessions";

import { creatingSlot } from "./held-slots";
import type { Session, Stopped } from "@armada/screens/src/draft/sessions";
import type { HeldOpenBy } from "@armada/protocol";
import { startsWhenSaid } from "@armada/screens/src/session-waits";

import { viewedAt } from "./cockpit/viewed";

/** Whether he has been in the Session since its turn ended on a reply. */
const readSince = (session: Session): boolean =>
  session.lastTurnAt !== undefined && viewedAt(session.id) >= Date.parse(session.lastTurnAt);

/** Whether the delivery question is what he is asked, so the Needs you card opens on it without a click. */
export const holdsDelivery = (session: Session): boolean =>
  owesOf(session).some((one) => one.source === "delivery");

/** One thing that keeps the Job open, as the words its mark's tooltip says. Fleet derives what; this only words it. */
function waitOf(one: HeldOpenBy): WaitingLine {
  if (one.kind === "watch")
    return {
      kind: "watch",
      said: `Held open by ${one.name}`,
    };
  const { number, on } = one;
  if (on.kind === "queue")
    return {
      kind: "merge",
      said: `#${number} in the merge queue`,
    };
  if (on.kind === "checks")
    return {
      kind: "checks",
      said: on.auto_merge
        ? `#${number} to auto-merge once Checks pass`
        : `Checks running on #${number}`,
    };
  return {
    kind: "merge",
    said: `#${number} to auto-merge`,
  };
}

/**
 * Everything a Job waits on that is not him, in the order Fleet derived it: each live watch, then each of its own pull requests in the
 * queue, set to merge, or on its Checks. Fleet sets it only once its turn has ended and nothing waits on him (`stateOf`).
 */
export function waitsOf(session: Session): WaitingLine[] {
  return (session.heldOpenBy ?? []).map(waitOf);
}

/** Where Fleet stands on bringing a stopped Job back, in its own words. None while it has not started or failed to. */
export const restartSaid = (stopped: Stopped): string | undefined =>
  stopped.restart === "queued"
    ? "Restart queued"
    : stopped.restart === "running"
      ? "Restarting"
      : stopped.restart === "failed"
        ? "Restart failed"
        : undefined;

/** Where a repair Job stands on a Job whose restart failed. */
export const repairSaid = (stopped: Stopped): string | undefined =>
  stopped.repair === "running"
    ? "Repair Job running"
    : stopped.repair === "failed"
      ? "Repair Job failed"
      : stopped.repair === "held"
        ? "No repair Job: Fleet is unhealthy"
        : undefined;

/** Why a Job stopped, as the first line of Now and the mark's tooltip say it. The tasks it left open are named in Now. */
export function stoppedSaid(stopped: Stopped): string {
  switch (stopped.reason) {
    case "turn_ended":
      return "Stopped: ended its turn with nothing running";
    case "tasks_open":
      return "Stopped: ended its turn with tasks open";
    case "process_gone":
      return "Stopped: process ended";
    case "fleet_down": {
      const clock = {
        ...timeFormat(),
        seconds: false,
        date: false,
      };
      return stopped.down === undefined
        ? "Stopped: Fleet was down"
        : `Stopped: Fleet was down, ${timeSaid(stopped.down.from, clock)} to ${timeSaid(stopped.down.to, clock)}`;
    }
    case "you":
      return "Stopped by you";
  }
}

/** Parked by him: nothing is asked of him, whatever it was waiting on, until he resumes it. An ended or unreachable Job reads as that. */
export function stateOf(session: Session): {
  state: SessionState;
  said: string;
} {
  if (session.paused === true && session.dead === undefined)
    return {
      state: "paused",
      said: "Paused",
    };
  if (session.dead === "ended")
    return {
      state: "ended",
      said: "Ended",
    };
  if (session.dead === "quiet")
    return {
      state: "quiet",
      said: "Not reachable",
    };
  // The agent process never came up: nothing runs, and nothing but a message from him moves it.
  if (session.startFailed !== undefined)
    return {
      state: "unstarted",
      said: `Failed to start: ${session.startFailed.cause}`,
    };
  // Not running, and not parked: it ended its turn with nothing running, its process ended, Fleet was down, or he stopped it. It raises no item: this mark and its call reach him, as stuck does.
  if (session.stopped !== undefined) {
    const restart = repairSaid(session.stopped) ?? restartSaid(session.stopped);
    return {
      // Amber only once Fleet has given up on bringing it back: while a nudge or a restart is coming, nothing is asked of him.
      state: session.stopped.gaveUp === true ? "stopped_asks" : "stopped",
      said:
        restart === undefined
          ? stoppedSaid(session.stopped)
          : `${stoppedSaid(session.stopped)} · ${restart}`,
    };
  }
  // Dispatched after another Session: nothing runs, and nothing is asked of him, until its condition is met.
  if (session.waits !== undefined)
    return {
      state: "after",
      said: startsWhenSaid(session.waits),
    };
  // The proposer is reading the request: the Session starts on its answer.
  if (session.work?.proposal?.reading === true)
    return {
      state: "working",
      said: "Reading the request",
    };
  // Read and held on its start card: nothing has moved, and nothing is asked of him but Start.
  if (session.start !== undefined)
    return {
      state: "draft",
      said: "Not started",
    };
  if (session.limitedUntil !== undefined)
    return {
      state: "limited",
      said: limitHeldSaid(session.limitedUntil),
    };
  // A turn held on a question or a permission is waiting on him, though the turn is open.
  if (session.asked !== undefined)
    return {
      state: "waiting",
      said: "Waiting on you",
    };
  // A full pool holds the agent's first write, the delivery question holds the agent at its start, a plan put to him
  // holds it until he answers, a sketch or a document put to him waits on his OK, and the done ask on his tasks is his to answer, whatever its turn says.
  if (
    owesOf(session).some(
      (one) =>
        (one.source === "slot_choice" && !creatingSlot(one)) ||
        one.source === "delivery" ||
        one.source === "plan" ||
        one.source === "task_done" ||
        one.source === "sketch" ||
        one.source === "document",
    )
  )
    return {
      state: "waiting",
      said: "Waiting on you",
    };
  const standing = standingOf(session);
  if (standing === "stuck")
    return {
      state: "stuck",
      said: "Stuck",
    };
  if (session.turn.state === "working") {
    const by = session.turn.wokenBy;
    return {
      state: "working",
      said: by === undefined ? "Working" : `Woken by ${by.title}`,
    };
  }
  // Fleet's word covers what a turn of its own does not: a subagent at work, a terminal's turn.
  if (standing === "working") {
    if (attachmentsOf(session, "subagent").some((one) => one.state === "running"))
      return {
        state: "subagent",
        said: "Subagent working",
      };
    const command = attachmentsOf(session, "background").find((one) => one.state === "running");
    const check = session.rows.find((row) => row.kind === "check" && row.state === "running");
    const running = command?.task ?? (check?.kind === "check" ? check.name : undefined);
    if (running !== undefined)
      // A reply that asks beside a command it left running is his to answer first.
      return owesOf(session).length > 0
        ? {
            state: "waiting",
            said: "Waiting on you",
          }
        : {
            state: "running",
            said: `Running: ${running}`,
          };
    return {
      state: "working",
      said: "Working",
    };
  }
  if (isBlank(session))
    return {
      state: "blank",
      said: "Blank: no slot, no branch",
    };
  // A question nobody answered in time, or kept for a walk, waits on him as an item does: Fleet keeps no item for it.
  if (owesOf(session).length > 0 || session.lapsed !== undefined)
    return {
      state: "waiting",
      said: "Waiting on you",
    };
  // Fleet lists what holds it open only while its turn has ended and nothing waits on him: a watch, or its own pull request in the queue, set to merge, or on its Checks.
  const waits = waitsOf(session);
  if (waits.length > 0)
    return {
      state: "held",
      said: waits.map((one) => one.said).join(" · "),
    };
  if (standing === "replied" && !readSince(session))
    return {
      state: "replied",
      said: "Unread reply",
    };
  if (standing === "done")
    return {
      state: "done",
      said: "Done",
    };
  const failing = attachmentsOf(session, "pull_request").find(
    (pr) => pr.state !== "merged" && pr.checks.state === "failed",
  );
  if (failing !== undefined)
    return {
      state: "failing",
      said: `Checks failed on #${failing.number}`,
    };
  return {
    state: "idle",
    said: "Idle",
  };
}
