// A Session in auto held on a read outside its slots: `~/Documents/handoffs/HANDOFF.md`, a folder macOS
// guards, by a command longer than Fleet's 200-character line, which its card draws whole. The card says
// it is a read and offers Allow once, Reject once, Always allow, Always allow for this session and
// Always block, the last three over a folder picker. Always allow on Documents settles
// the card and puts Documents on the list Settings, Features, Folders sessions reach shows, beside the
// blocked `~/.ssh` already there. A second Session is held on a write to `~/Desktop/drafts/notes.md`,
// whose card says it is a write; Always allow for this session settles it and lists nothing. #2441.
// Mock data only.

import type { HelmCallInFlight, RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { FakeSessionsFleet, held, hosted } from "../sessions-fleet";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };
const SESSION = "01SESSIONREACHOUTSIDEAAAAA";
const WRITER = "01SESSIONREACHWRITESAAAAAA";
const AT = "2026-10-16T13:05:00.000Z";
const PATH = "/Users/user/Documents/handoffs/HANDOFF.md";

// Longer than the 200 characters `detail` keeps, so Fleet cuts it and sends it whole beside.
const READING = `cat ${PATH} | sed -n '1,120p' | grep -v '^<!--' | awk 'NF' > /tmp/handoff.txt && wc -l /tmp/handoff.txt && grep -n 'margin' /tmp/handoff.txt | head -n 20 && tail -n 30 /tmp/handoff.txt && echo gutterline`;

const reaching: HelmCallInFlight = {
  call: "helm-7",
  manifest_id: "armada",
  asked_at: AT,
  tool: "Bash",
  detail: READING.slice(0, 200),
  truncated: true,
  length: READING.length,
  whole: READING,
  rule: "Bash(cat:*)",
  offers: ["allow_once", "refuse", "always_allow", "allow_for_session", "always_block"],
  holding_for_seconds: 300,
  reaches: PATH,
  reaches_to: "read",
};

const writing: HelmCallInFlight = {
  call: "helm-8",
  manifest_id: "armada",
  asked_at: AT,
  tool: "Write",
  detail: "/Users/user/Desktop/drafts/notes.md",
  truncated: false,
  rule: "Write",
  offers: ["allow_once", "refuse", "always_allow", "allow_for_session", "always_block"],
  holding_for_seconds: 300,
  reaches: "/Users/user/Desktop/drafts/notes.md",
  reaches_to: "write",
};

function fleet(): FakeSessionsFleet {
  const records = [
    hosted(SESSION, { title: "Custom margins", attachments: [held("slot", "4")], hosted: { turn: { state: "working" }, mode: "auto", running: true, asked: reaching } }),
    hosted(WRITER, { title: "Release notes", attachments: [held("slot", "5")], hosted: { turn: { state: "working" }, mode: "auto", running: true, asked: writing } }),
  ];
  const thread = {
    [SESSION]: [
      { kind: "message" as const, id: "m1", at: AT, from: { kind: "you" as const }, text: "The handoff is in ~/Documents/handoffs/HANDOFF.md. Read it and carry on." },
      { kind: "ask" as const, id: `ask-${reaching.call}-${SESSION}`, at: AT, ask: reaching, state: "waiting" as const },
    ],
    [WRITER]: [
      { kind: "message" as const, id: "m2", at: AT, from: { kind: "you" as const }, text: "Draft the release notes on my Desktop, in drafts/notes.md." },
      { kind: "ask" as const, id: `ask-${writing.call}-${WRITER}`, at: AT, ask: writing, state: "waiting" as const },
    ],
  };
  return new FakeSessionsFleet(records, thread);
}

function build(): Scenario {
  const board = onBoard({
    repositories: [ARMADA],
    picked: ARMADA.root,
  });
  return {
    ...fleet().scenario(board),
    name: "session-reach-outside",
    says: "A Session's read and another's write outside their slots, each put to you with a folder picker, and the folder list in Settings",
    behaves: (handle) => fleet().scenario(board).behaves!(handle),
  };
}

export const sessionReachOutside: Scenario = build();
