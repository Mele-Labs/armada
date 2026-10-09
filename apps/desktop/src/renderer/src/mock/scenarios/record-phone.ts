// A Job's Record with acts from two doors: the approval and a redirect taken on the phone, the
// send-back taken in Bridge. The phone rows draw a smartphone glyph beside Who, tooltip From phone.
// Mock data only. Open the Job, then its Record tab.

import type { Recorded } from "@armada/protocol";
import { reviewHeldByPolicy } from "@armada/jobs/fixtures/build/policy";

import { asRow, holding } from "../holding";

const fixture = asRow(reviewHeldByPolicy(), 91, "record-phone", "Pin the store clock");

const history = fixture.history;
const moves: Recorded[] =
  history === undefined || history.state !== "read"
    ? []
    : history.moves.map((move) =>
        move.seq === 1 ? { ...move, via: "phone" } : move.seq === 12 ? { ...move, via: "bridge" } : move,
      );

const redirect: Recorded = {
  seq: 19,
  status: "escalated",
  moved: { kind: "status", to: "running" },
  actor: "human",
  at: "2026-09-10T14:31:20Z",
  via: "phone",
};

const phoned =
  fixture.history === undefined || fixture.history.state !== "read"
    ? fixture
    : { ...fixture, history: { ...fixture.history, moves: [...moves, redirect] } };

export const s223RecordPhone = holding(
  "record-phone",
  "A Job's Record with an approve and a redirect from the phone beside a send-back from Bridge",
  [phoned],
  { opens: phoned.job.id },
);
