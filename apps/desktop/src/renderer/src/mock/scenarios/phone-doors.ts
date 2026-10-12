// The three acts that leave no move to hang a door on, each taken from the phone: a Job dispatched,
// a Session's ask answered, a redirect into a Drone that is not stuck. Each draws the smartphone glyph,
// tooltip From phone; the same act in Bridge draws none. Mock data only.
//
// Job "Pin the store clock" (awaiting approval): open it, then its Record tab. The created row is marked.
// Job "Cache the manifest read" (running): open it, Drones, then the step's log. The redirect row is marked.
// Session "Fix the flaky store test": open it, then the Bash calls fold. Two asks: the phone's is marked.

import { asRow, holding } from "../holding";
import { awaitingApproval, running } from "@armada/jobs/fixtures/build/index";
import type { Turn } from "@armada/protocol";

import { FakeSessionsFleet, asking, hosted } from "../sessions-fleet";

const SESSION = "01SESSIONPHONEDOORSAAAAAAA";

const phoned = (() => {
  const fixture = asRow(awaitingApproval(), 92, "phone-created", "Pin the store clock");
  if (fixture.watched.state !== "read") return fixture;
  return { ...fixture, watched: { ...fixture.watched, detail: { ...fixture.watched.detail, created_via: "phone" } } };
})();

const redirected = (() => {
  const fixture = asRow(running(), 93, "phone-redirect", "Cache the manifest read");
  if (fixture.observed.state !== "watching") return fixture;
  const turns = fixture.observed.turns;
  const redirect = (seq: number, ts: string, via: string): Turn => ({
    ts,
    seq,
    step: "fix",
    by: "armada",
    saw: { event: "instructed", occasion: "redirect", text: "Read the failing test before writing anything.", via },
  });
  return {
    ...fixture,
    observed: {
      ...fixture.observed,
      turns: { ...turns, rows: [...turns.rows, redirect(9001, "2026-09-10T14:25:00Z", "phone"), redirect(9002, "2026-09-10T14:26:00Z", "bridge")] },
    },
  };
})();

const base = holding("phone-doors", "A Job dispatched, a Session ask and a redirect, each from the phone beside the same act in Bridge", [phoned, redirected], {
  opens: phoned.job.id,
});

const AT = "2026-10-11T09:00:00.000Z";
const thread = {
  [SESSION]: [
    { kind: "ask" as const, id: "ask-1", at: AT, ask: asking("rm -rf build", "call-1"), state: "allowed_once" as const, via: "phone" },
    { kind: "ask" as const, id: "ask-2", at: AT, ask: asking("rm -rf dist", "call-2"), state: "allowed_once" as const },
  ],
};

export const s224PhoneDoors = new FakeSessionsFleet([hosted(SESSION, { title: "Fix the flaky store test" })], thread).scenario(base);
