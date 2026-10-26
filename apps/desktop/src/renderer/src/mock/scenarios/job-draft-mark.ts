// An agent has started a Job, and Fleet holds it on its start card: the agent's first message sits in the Job's
// box and nothing runs until he presses Start. The walk `job-draft-mark` plays the mark the Job wears until
// then. The Job is already started when the window opens, so the walk goes straight to its row.

import type { RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { FakeSessionsFleet, held, hosted } from "../sessions-fleet";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

const AT = "2026-10-12T09:14:02.000Z";
const DEBUG = "01DEBUGCAPAAAAAAAAAAAAAAAA";

const START = "Read crates/settings/src/loader.rs and find out whether a cap set for one repository is ever read. It was set to 3 and the loop ran 5 attempts. Report where the value is dropped.";

/** A Fleet holding the started Job on its start card, whose agent works once Start is pressed. */
function fleet(): FakeSessionsFleet {
  const served = new FakeSessionsFleet(
    [hosted(DEBUG, { title: "Debug the retry cap", last_turn_at: AT, attachments: [held("slot", "3"), held("branch", "fix/retry-cap")] })],
    {
      [DEBUG]: [
        { kind: "message", id: "m1", at: AT, from: { kind: "you" }, text: "The retry loop ran 5 attempts with the cap set to 3. Find out why." },
        { kind: "message", id: "m2", at: AT, from: { kind: "agent" }, text: "The settings loader seems to ignore a cap set for one repository." },
      ],
    },
  );
  const started = served.agentStarts(DEBUG, { title: "Settings loader and the cap", message: START });
  served.onSend = (send) => {
    if (send.session_id !== started) return;
    void served.agentWorks(started, { kind: "tool", id: "s1", at: AT, text: "Read crates/settings/src/loader.rs" });
  };
  return served;
}

function build(): Scenario {
  const board = onBoard({ repositories: [ARMADA], picked: ARMADA.root });
  const served = fleet().scenario(board);
  return {
    ...served,
    name: "job-draft-mark",
    says: "An agent has started a Job, held on its start card until the owner presses Start",
    // A Fleet of its own for each window, so a walk told twice starts from the same draft.
    behaves: (handle) => fleet().scenario(board).behaves!(handle),
  };
}

export const s204JobDraftMark: Scenario = build();
