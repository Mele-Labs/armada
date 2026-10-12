// A Job's Commander, drawn on Job 14's shape: "Wire the Cockpit's Job detail panel and Job call cards
// to real Fleet data", feature workflow, Implement, ten tasks, T7 at work. Three moments of one Job:
//   woken: T7's Drone ends at the turn cap, Fleet wakes the Commander, and it steers with nobody asked.
//   asked: T7's Drone sits stuck, and the owner asks the Commander why and then to fix it.
//   asks:  Group 3 is red on a Check no change of the Job's can turn green, so it asks him with one card.
// Mock data only: Fleet serves no Commander, so the thread, its acts and the panels ride on the draft.
// The walks `job-commander-*` play them, one per scenario.

import type { CallView } from "@armada/jobs/draft/calls";
import type { NowRunningView, NowView } from "@armada/jobs/draft/now";
import { commanderJob, COMMANDER_REQUEST } from "@armada/jobs/fake";
import type { CommanderStage } from "@armada/jobs/fake";

import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import { commandedBy, failedTask, groupState, runsCleared, splitPlan, taskStates } from "../commander-fleet";
import type { Commanding } from "../commander-fleet";
import { asRow, holding } from "../holding";
import type { Scenario } from "../moment";

const ROW = { at: 14, slug: "commander", title: COMMANDER_REQUEST } as const;

/** The recording's Job was cleared after it ran; this one is on the Board's Running section. */
function uncleared(fixture: JobFixture): JobFixture {
  const { reclaimed_at: _cleared, ...job } = fixture.job;
  if (fixture.watched.state !== "read") return { ...fixture, job };
  const { reclaimed_at: _also, ...detailJob } = fixture.watched.detail.job;
  return { ...fixture, job, watched: { ...fixture.watched, detail: { ...fixture.watched.detail, job: detailJob } } };
}

const fixtureAt = (stage: CommanderStage) => uncleared(asRow(commanderJob(stage), ROW.at, ROW.slug, ROW.title));

const IMPLEMENT = { id: "implement", name: "Implement" } as const;
const DRONE_ID = "01M3WJ6FGZ003DCX123T7W6YP1";

/** T7's Drone at work, and who put it to work where that was the Commander. */
const droneOn = (line: string, tail: readonly string[], by?: string): NowRunningView => ({
  key: "d7",
  of: "drone",
  name: "Implement Drone",
  line,
  step: IMPLEMENT,
  tail,
  state: "running",
  target: DRONE_ID,
  ...(by === undefined ? {} : { by: { who: "commander" as const, said: by } }),
});

const CHECK_ACTS = [
  { key: "retry", glyph: "retry", said: "Retry now" },
  { key: "skip", glyph: "skip", said: "Skip check" },
] as const;

// ---- woken ---------------------------------------------------------------------------------------

const T7_AT_WORK = droneOn("Edit crates/ipc/src/asks.rs", ["Read crates/ipc/src/asks.rs", "Edit crates/ipc/src/asks.rs", "Edit apps/desktop/src/main/asks.ts"]);

const woken = commandedBy({
  job: fixtureAt("working").job.id,
  open: (c) => c.panels({ now: { running: [T7_AT_WORK] } }),
  moments: [
    // T7's Drone ends at the cap with nothing handed in: Fleet wakes the Commander.
    (c) => {
      c.plan((whole) => failedTask(whole, "T7", "Drone ended at the turn cap with nothing handed in"));
      c.panels({ now: { issues: [{ key: "i7", of: "drone", name: "Implement Drone", text: "Ended at the turn cap, nothing handed in", said: "Drone ended", step: IMPLEMENT, target: DRONE_ID }] } });
      c.wake("Drone on T7 ended with nothing handed in");
    },
    // It reads why, and acts inside its reach: no card, nobody asked.
    (c) => {
      c.read("get_drone T7");
      c.read("get_job_log");
      c.read("get_diff");
      c.say("T7's Drone ended at the 300 turn cap with six files edited and nothing handed in. The task reaches the wire, main and the Cockpit card, which is more than one Drone holds in 300 turns. I split the card half off as T11, raised the cap to 600 and restarted T7.");
      c.act({ kind: "raise_cap", cap: "turns", from: "300", to: "600" });
      c.plan((whole) =>
        splitPlan(
          whole,
          "T7",
          { title: "Serve a Drone's permission ask from Fleet", scope: ["crates/ipc/src/asks.rs", "apps/desktop/src/main/asks.ts"] },
          {
            id: "T11",
            title: "Draw a Drone's permission ask on the call card",
            scope: ["packages/surfaces/jobs/src/now-real.ts", "apps/desktop/src/renderer/src/cockpit/CallCard.tsx"],
            state: "open",
            group: "G3",
          },
        ),
      );
      c.act({ kind: "split", task: "T7", into: ["T7", "T11"] });
      c.mark("T7", "Commander split T11 off it");
      c.mark("T11", "Commander split it off T7");
      c.plan((whole) => taskStates(whole, { T7: "working" }));
      c.act({ kind: "restart", target: "T7" });
      c.panels({ now: { running: [droneOn("Read crates/ipc/src/asks.rs", ["Read crates/ipc/src/asks.rs"], "Commander restarted T7")] } });
      c.state("idle");
    },
  ],
});

// ---- asked ---------------------------------------------------------------------------------------

const STUCK: NowView = {
  issues: [
    {
      key: "i7",
      of: "drone",
      name: "Implement Drone",
      text: "No output for 14m",
      said: "Drone stuck",
      step: IMPLEMENT,
      acts: [
        { key: "redirect", glyph: "redirect", said: "Redirect Drone" },
        { key: "restart", glyph: "retry_step", said: "Retry step" },
      ],
      target: DRONE_ID,
    },
  ],
};

const asked = commandedBy({
  job: fixtureAt("stuck").job.id,
  open: (c) => c.panels({ now: STUCK }),
  replies: [
    // Why did it stop?
    (c) => {
      c.read("get_drone T7");
      c.read("get_job_log");
      c.say("T7's Drone has sat at turn 288 of 300 for 14 minutes. Its last call is cargo test, waiting on a target lock another Job's build holds. It reaches the cap before the lock frees.");
      c.state("idle");
    },
    // Fix it.
    (c) => {
      c.act({ kind: "raise_cap", cap: "turns", from: "300", to: "600" });
      c.mark("T7", "Commander restarted it");
      c.act({ kind: "restart", target: "T7" });
      c.panels({ now: { running: [droneOn("Read crates/ipc/src/asks.rs", ["Read crates/ipc/src/asks.rs"], "Commander restarted T7")] } });
      c.say("T7 has a new Drone and a cap of 600.");
      c.state("idle");
    },
  ],
});

// ---- asks ----------------------------------------------------------------------------------------

const QUESTION = "What should Group 3 do about desktop_test?";
const OPTIONS = [
  { id: "skip", label: "Skip desktop_test on Group 3" },
  { id: "edit", label: "Edit the Check in the Manifest" },
] as const;

const RED_NOW: NowView = {
  asks: [{ key: "card", kind: "commander", text: "desktop_test is red on main as well", reach: "A Check is skipped or edited by you", question: QUESTION, options: OPTIONS }],
  issues: [{ key: "c-desktop", of: "check", name: "desktop_test", text: "desktop_test: 3 failed", said: "Check failed", step: IMPLEMENT, acts: CHECK_ACTS, target: "desktop_test" }],
};

const redCalls = (c: Commanding): CallView => ({
  request: COMMANDER_REQUEST,
  asks: [{ key: "card", kind: "commander", context: ["desktop_test: 3 of 2210 failed on every run", "The same 3 fail on main"], decisions: [{ id: "check", question: QUESTION, options: OPTIONS }], onAnswer: c.answer }],
});

const asks = commandedBy({
  job: fixtureAt("red").job.id,
  open: (c) => {
    c.panels({ now: RED_NOW, calls: redCalls(c) });
    c.wake("Group 3 red with its retries spent");
    c.read("get_check_output desktop_test");
    c.read("get_diff");
    c.say("Group 3 failed desktop_test on all three runs. The same 3 tests fail on main, in code the Job did not touch, so nothing inside it turns the Check green. Skipping or editing a Check is not mine to do. The card is yours.");
    c.state("waiting");
  },
  answered: (c, option) => {
    const picked = OPTIONS.find((one) => one.id === option)?.label ?? option;
    c.wake(`You answered: ${picked}`);
    if (option !== "skip") {
      c.say("Group 3 stays red until the Check is edited.");
      c.panels({ now: { issues: RED_NOW.issues ?? [] }, calls: {} });
      c.state("idle");
      return;
    }
    c.read("get_job");
    c.plan((whole) => groupState(runsCleared(taskStates(whole, { T9: "working" }), "G3"), "G3", "running"));
    c.act({ kind: "restart", target: "T9" });
    c.mark("T9", "Commander restarted it");
    c.panels({ now: { running: [droneOn("Read apps/desktop/src/renderer/src/cockpit/CallCard.tsx", ["Read apps/desktop/src/renderer/src/cockpit/CallCard.tsx"], "Commander restarted T9")] }, calls: {} });
    c.state("idle");
  },
});

// ---- the scenarios -------------------------------------------------------------------------------

function opening(name: string, says: string, stage: CommanderStage, commanding: ReturnType<typeof commandedBy>): Scenario {
  const fixture = fixtureAt(stage);
  const row = holding(`job-commander-${name}`, `A Job's Commander: ${says}`, [fixture], { opens: fixture.job.id });
  return { ...row, draft: { commanding } };
}

export const s230JobCommanderWoken = opening("woken", "Fleet wakes it when T7's Drone ends, and it steers with nobody asked", "working", woken);
export const s231JobCommanderAsked = opening("asked", "the owner asks why T7's Drone stopped, then to fix it", "stuck", asked);
export const s232JobCommanderAsks = opening("asks", "a Check no change can fix, put to the owner as one card", "red", asks);
