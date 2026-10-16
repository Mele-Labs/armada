// A Session in auto held on a read outside its slots: `~/Documents/handoffs/HANDOFF.md`, a folder macOS
// guards. Its card offers Allow once, Reject once, Always allow and Always block, the last two over a
// folder picker. Always allow on Documents settles the card and puts Documents on the list Settings,
// Features, Folders sessions reach shows, beside the blocked `~/.ssh` already there. #2441. Mock data only.

import type { HelmCallInFlight, RepositorySummary } from "@armada/protocol";
import { job, repository } from "@armada/screens/src/fixtures/build/base";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { FakeSessionsFleet, held, hosted } from "../sessions-fleet";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };
const SESSION = "01SESSIONREACHOUTSIDEAAAAA";
const AT = "2026-10-16T13:05:00.000Z";
const PATH = "/Users/user/Documents/handoffs/HANDOFF.md";

const reaching: HelmCallInFlight = {
  call: "helm-7",
  manifest_id: "armada",
  asked_at: AT,
  tool: "Read",
  detail: "~/Documents/handoffs/HANDOFF.md",
  truncated: false,
  rule: "Read",
  offers: ["allow_once", "refuse", "always_allow", "always_block"],
  holding_for_seconds: 300,
  reaches: PATH,
};

function fleet(): FakeSessionsFleet {
  const records = [hosted(SESSION, { title: "Custom margins", attachments: [held("slot", "4")], hosted: { turn: { state: "working" }, mode: "auto", running: true, asked: reaching } })];
  const thread = {
    [SESSION]: [
      { kind: "message" as const, id: "m1", at: AT, from: { kind: "you" as const }, text: "The handoff is in ~/Documents/handoffs/HANDOFF.md. Read it and carry on." },
      { kind: "ask" as const, id: `ask-${reaching.call}-${SESSION}`, at: AT, ask: reaching, state: "waiting" as const },
    ],
  };
  return new FakeSessionsFleet(records, thread);
}

function build(): Scenario {
  const board = onBoard([job("running", { id: "01JOBREACHRUNNING", handle: "70-custom-margins", title: "Custom margins", branch: "bridge/custom-margins", owner_manifest_id: "armada" })], {
    repositories: [ARMADA],
    picked: ARMADA.root,
  });
  return {
    ...fleet().scenario(board),
    name: "session-reach-outside",
    says: "A Session's read outside its slots, put to you with a folder picker and the folder list in Settings",
    behaves: (handle) => fleet().scenario(board).behaves!(handle),
  };
}

export const sessionReachOutside: Scenario = build();
