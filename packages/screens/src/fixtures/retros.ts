// Two Sessions with their retros written, for the Lessons page and the Retros sheet. The newer is
// a retro as the model writes one now, every item with a title, a what and a fix. The older was
// kept before an item had any of the three, so its one item has a statement alone.

import type { Lesson, Retro } from "@armada/protocol";

import { sessionRetro } from "../lessons/session-retro";

const WRITTEN_NEWER = "2026-10-02T22:05:00.000Z";
const WRITTEN_OLDER = "2026-10-01T21:10:00.000Z";

const NEWER = { id: "01M2SESSIONRETRO3WRITTEN", title: "Retire guides 8 and 20, add validation that" };
const OLDER = { id: "01M2SESSIONRETRO2WRITTEN", title: "Pin the Bridge store clock" };

/** The older Session's retro: written before an item had a title, a what or a fix. */
function olderRetro(): { retro: Retro; lessons: Lesson[] } {
  const item = {
    id: "01M2LESSON2DOCKWAIT",
    who: "owner" as const,
    statement: "The question waited in the dock while the plan was read twice.",
    evidence: ["ask:1"],
  };
  const retro: Retro = {
    session: OLDER,
    state: "written",
    at: WRITTEN_OLDER,
    items: [item],
    record: {
      asks: [
        {
          cite: "ask:1",
          asked_at: "2026-10-01T20:26:21.196Z",
          about: "Which plan should I follow?",
          answered_at: "2026-10-01T20:31:04.000Z",
          answer: "The second",
          waited_ms: 282_804,
        },
      ],
    },
  };
  return { retro, lessons: [{ handle: OLDER.id, session: OLDER, at: WRITTEN_OLDER, ...item, state: "open" }] };
}

/** Both Sessions' retros by Session id, and the Lessons listing over both, newest retro first, every item still open. */
export function retroFixtures(): { retros: Record<string, Retro>; lessons: Lesson[] } {
  const newer = sessionRetro(NEWER, WRITTEN_NEWER);
  const older = olderRetro();
  return {
    // The newer is read by its retro number as the listing names it, and the older, whose item has none, by its Session.
    retros: { [NEWER.id]: newer.retro, [`${NEWER.id}-r1`]: newer.retro, [OLDER.id]: older.retro },
    lessons: [...newer.lessons, ...older.lessons],
  };
}
