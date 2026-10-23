// A Job whose agent writes a Jira key and proposes an idea in prose. Selecting words in either offers Teach: the owner says what
// the words are, Armada shows back what it understood, and Save keeps it. The walk `session-teach` plays it.

import type { RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { FakeSessionsFleet } from "../sessions-fleet";
import { teachSessions, teachThreads } from "../sessions/teach";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

function build(): Scenario {
  const board = onBoard({ repositories: [ARMADA], picked: ARMADA.root });
  const served = new FakeSessionsFleet(teachSessions(), teachThreads()).scenario(board);
  return {
    ...served,
    name: "session-teach",
    says: "Selected words in a message, taught to Armada as a link and as a habit for agents",
    // A Fleet of its own for each window, so a walk told twice starts from the same Job.
    behaves: (handle) => new FakeSessionsFleet(teachSessions(), teachThreads()).scenario(board).behaves!(handle),
  };
}

export const sessionTeach: Scenario = build();
