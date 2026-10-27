// A job in auto held on a Bash write outside its slots, `node drive.mjs --out ~/Desktop/shot.png`, whose
// card names the call's kind and offers "Allow calls like this for this job" in a Calls like row. Allowed
// for the script, the agent's next drive.mjs call runs with no card, and once time passes its cleanup.mjs
// call is held on a new card naming cleanup.mjs. Allowed for every node call, both run. Mock data only.

import type { HelmCallInFlight, RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { FakeSessionsFleet, held, hosted } from "../sessions-fleet";
import { onTimePassing } from "../time-passes";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };
const JOB = "01SESSIONCALLSLIKETHISAAAA";
const AT = "2026-10-26T15:20:00.000Z";
const SHOT = "/Users/user/Desktop/shot.png";
const DRIVING = `node drive.mjs --out ${SHOT}`;
const BROWSER = "node drive.mjs --browser /Users/user/Library/Caches/ms-playwright/chromium/chrome";
const CLEANUP = "node cleanup.mjs /Users/user/code/other-repo";

const call = (id: string, command: string, reaches: string, script: string): HelmCallInFlight => ({
  call: id,
  manifest_id: "armada",
  asked_at: AT,
  tool: "Bash",
  detail: command,
  truncated: false,
  rule: "Bash(node:*)",
  offers: ["allow_once", "refuse", "always_allow", "allow_for_session", "allow_like_this_for_session", "always_block"],
  holding_for_seconds: 300,
  reaches,
  reaches_to: "write",
  kind: { program: "node", script },
});

const driving = call("helm-21", DRIVING, SHOT, "drive.mjs");
const cleaning = call("helm-22", CLEANUP, "/Users/user/code/other-repo", "cleanup.mjs");

function fleet(): FakeSessionsFleet {
  const records = [hosted(JOB, { title: "Browser checks", attachments: [held("slot", "6")], hosted: { turn: { state: "working" }, mode: "auto", running: true, asked: driving } })];
  const thread = {
    [JOB]: [
      { kind: "message" as const, id: "m1", at: AT, from: { kind: "you" as const }, text: "Run the browser checks and save a screenshot to my Desktop." },
      { kind: "ask" as const, id: `ask-${driving.call}-${JOB}`, at: AT, ask: driving, state: "waiting" as const },
    ],
  };
  const served = new FakeSessionsFleet(records, thread);
  served.onAnswer = (answer) => {
    if (answer.session_id !== JOB || answer.call !== driving.call || answer.answer !== "allow_like_this_for_session") return;
    // The script's kind covers the next drive.mjs call and not cleanup.mjs; the program's covers both.
    const wide = answer.kind?.script === undefined;
    const ran = { kind: "tool", id: "t1", at: AT, text: `Bash ${BROWSER}` } as const;
    if (wide) void served.agentTurn(JOB, [ran, { kind: "tool", id: "t2", at: AT, text: `Bash ${CLEANUP}` }]);
    // Held for a walk's `later` step, so the call that ran is read before a new card covers the thread.
    else void served.agentTurn(JOB, [ran]).then(() => onTimePassing(() => served.asks(JOB, cleaning)));
  };
  return served;
}

function build(): Scenario {
  const board = onBoard({
    repositories: [ARMADA],
    picked: ARMADA.root,
  });
  return {
    ...fleet().scenario(board),
    name: "calls-like-this",
    says: "A job's Bash write outside its slots, allowed for calls like it, and the next call of another script held on a new card",
    // A Fleet of its own for each window, so a walk told twice starts held on the same write.
    behaves: (handle) => fleet().scenario(board).behaves!(handle),
  };
}

export const callsLikeThis: Scenario = build();
