// A Session's agent starts a Session, and forks its own conversation, when the owner asks it to. Either
// waits for him: held on its start card with the agent's first message in the box, and nothing runs until
// he presses Start. The walk `session-agent-starts` plays it, over a Fleet that holds the start as Fleet
// would (`../sessions-fleet.ts`), with the agent's turns scripted on what he types.

import type { RepositorySummary, SessionRow } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { FakeSessionsFleet, held, hosted } from "../sessions-fleet";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

const AT = "2026-10-12T09:14:02.000Z";
const DEBUG = "01DEBUGCAPAAAAAAAAAAAAAAAA";

const records = () => [
  hosted(DEBUG, { title: "Debug the retry cap", attachments: [held("slot", "3"), held("branch", "fix/retry-cap")] }),
  hosted("01SESSIONBBBBBBBBBBBBBBBBB", { title: "Fix the flaky store test", attachments: [held("slot", "4"), held("branch", "fix/flaky-store")] }),
];

const threads = (): Record<string, SessionRow[]> => ({
  [DEBUG]: [
    { kind: "message", id: "m1", at: AT, from: { kind: "you" }, text: "The retry loop ran 5 attempts with the cap set to 3. Find out why." },
    { kind: "tool", id: "t1", at: AT, text: "Read crates/retry/src/loop.rs" },
    { kind: "tool", id: "t2", at: AT, text: "Bash cargo test -p retry cap" },
    {
      kind: "message",
      id: "m2",
      at: AT,
      from: { kind: "agent" },
      text: "The loop reads the cap once at startup, so the lowered value never reaches the step. That is why the third attempt ran twice. Separately, the settings loader seems to ignore a cap set for one repository.",
    },
  ],
});

const START = "Read crates/settings/src/loader.rs and find out whether a cap set for one repository is ever read. It was set to 3 and the loop ran 5 attempts. Report where the value is dropped.";
const FORK = "Check the same cap where the loop reads it on every attempt and not at startup. Does the third attempt still run twice?";

/** A Fleet whose agent starts and forks on the owner's word, and works once its first message is sent. */
function fleet(): FakeSessionsFleet {
  const served = new FakeSessionsFleet(records(), threads());
  const started = new Set<string>();
  served.onSend = (send) => {
    const said = send.text.toLowerCase();
    if (started.has(send.session_id)) {
      void served.agentTurn(send.session_id, [
        { kind: "tool", id: "s1", at: AT, text: "Read crates/settings/src/loader.rs" },
        { kind: "message", id: "s2", at: AT, from: { kind: "agent" }, text: "The loader reads the cap from the manifest only. A cap set for one repository never reaches it." },
      ]);
    } else if (send.session_id === DEBUG && said.includes("start a session")) {
      void served.agentTurn(DEBUG, [
        { kind: "tool", id: "a1", at: AT, text: "Start session Settings loader and the cap" },
        () => {
          const title = "Settings loader and the cap";
          const id = served.agentStarts(DEBUG, { title, message: START });
          started.add(id);
          return { kind: "started", id: "a2", at: AT, session: { id, title }, fork: false };
        },
      ]);
    } else if (send.session_id === DEBUG && said.includes("fork")) {
      void served.agentTurn(DEBUG, [
        { kind: "tool", id: "a3", at: AT, text: "Fork session at the cap finding" },
        () => {
          const title = "Debug the retry cap, cap per attempt";
          const id = served.agentStarts(DEBUG, { title, message: FORK, fork: { at: "m2" } });
          started.add(id);
          return { kind: "started", id: "a4", at: AT, session: { id, title }, fork: true };
        },
      ]);
    }
  };
  return served;
}

function build(): Scenario {
  const board = onBoard({
    repositories: [ARMADA],
    picked: ARMADA.root,
  });
  const served = fleet().scenario(board);
  return {
    ...served,
    name: "session-agent-starts",
    says: "A Session's agent starts a Session and forks itself when asked, each held for the owner to send",
    // A Fleet of its own for each window, so a walk told twice starts from the same two Sessions.
    behaves: (handle) => fleet().scenario(board).behaves!(handle),
  };
}

export const s203SessionAgentStarts: Scenario = build();
