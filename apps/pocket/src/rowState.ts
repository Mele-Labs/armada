// What a row on Home says of a Job. Rows hold what the Gateway lists, not the whole Job, so this reads those fields in
// `jobStateOf`'s order, with its words.

import { limitHeldSaid, type SessionState } from "@armada/components";
import type { HeldOpenBy } from "@armada/protocol";
import { repairSaid, restartSaid, stoppedSaid, waitOf } from "@armada/screens/src/session-state";
import { startsWhenSaid } from "@armada/screens/src/session-waits";

import type { PhoneHold, PhoneSession } from "./gateway";

const heldOf = (one: PhoneHold): HeldOpenBy =>
  one.kind !== "pull_request"
    ? one
    : {
        kind: "pull_request",
        number: one.number,
        on:
          one.on === "checks"
            ? {
                kind: "checks",
                auto_merge: one.auto_merge === true,
              }
            : {
                kind: one.on,
              },
      };

/**
 * Three of `jobStateOf`'s answers read as their neighbours here. A reply reads as idle, since the phone keeps no record
 * of what he read, and a subagent or a command at work reads as working, since what runs stays on the Mac.
 */
export function rowStateOf(session: PhoneSession): {
  state: SessionState;
  said: string;
} {
  if (session.paused === true)
    return {
      state: "paused",
      said: "Paused",
    };
  if (session.start_failed !== undefined)
    return {
      state: "unstarted",
      said: `Failed to start: ${session.start_failed}`,
    };
  if (session.stopped !== undefined) {
    const { gave_up, ...stopped } = session.stopped;
    const named = {
      ...stopped,
      at: "",
    };
    const restart = repairSaid(named) ?? restartSaid(named);
    return {
      state: gave_up === true ? "stopped_asks" : "stopped",
      said: restart === undefined ? stoppedSaid(named) : `${stoppedSaid(named)} · ${restart}`,
    };
  }
  if (session.starts_when !== undefined) {
    const { on, when, part_title, pull_number } = session.starts_when;
    return {
      state: "after",
      said: startsWhenSaid({
        on,
        when,
        ...(part_title === undefined
          ? {}
          : {
              part: {
                id: "",
                title: part_title,
              },
            }),
        ...(pull_number === undefined
          ? {}
          : {
              pull: {
                number: pull_number,
                branch: "",
              },
            }),
      }),
    };
  }
  if (session.reading === true)
    return {
      state: "working",
      said: "Reading the request",
    };
  if (session.held)
    return {
      state: "draft",
      said: "Not started",
    };
  if (session.limited_until !== undefined)
    return {
      state: "limited",
      said: limitHeldSaid(session.limited_until),
    };
  if (session.waiting)
    return {
      state: "waiting",
      said: "Waiting on you",
    };
  if (session.standing === "stuck")
    return {
      state: "stuck",
      said: "Stuck",
    };
  if (session.working)
    return {
      state: "working",
      said: "Working",
    };
  if (session.blank === true)
    return {
      state: "blank",
      said: "Blank: no slot, no branch",
    };
  const holds = session.held_open_by ?? [];
  if (holds.length > 0)
    return {
      state: "held",
      said: holds.map((one) => waitOf(heldOf(one)).said).join(" · "),
    };
  if (
    session.standing === "done" ||
    (session.standing === undefined && session.done_since !== undefined)
  )
    return {
      state: "done",
      said: "Done",
    };
  if (session.failing_pull !== undefined)
    return {
      state: "failing",
      said: `Checks failed on #${session.failing_pull}`,
    };
  return {
    state: "idle",
    said: "Idle",
  };
}
