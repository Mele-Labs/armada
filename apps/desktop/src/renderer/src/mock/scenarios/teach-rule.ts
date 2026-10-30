// Teaching Armada a lifecycle rule from a pull request: one Job in one repository, holding a pull request in its Artifacts. Teach on the
// pull request reads "when this is approved, move its tickets to Done" as a rule, shown with its When, If and Do to change before it is
// kept. The lifecycles Fleet keeps for the repository are the pickers' (`../pieces-seed`), and the mock Fleet in `../teach` reads and keeps
// the rule over the same wire a real one has. The walk `teach-rule` plays it.

import type { RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { builtInPieces } from "../pieces-seed";
import { FakeSessionsFleet } from "../sessions-fleet";
import { ruleRecord, ruleThreads } from "../sessions/teach-rule";
import { bindTeachFleet, teachedMods } from "../teach";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

export function build(): Scenario {
  const board = onBoard({ repositories: [ARMADA] });
  const served = new FakeSessionsFleet([ruleRecord()], ruleThreads()).scenario(board);
  return {
    ...served,
    name: "teach-rule",
    says: "A rule taught from a pull request: when it is approved, move its tickets to Done, shown to change before it is kept",
    draft: { pieces: builtInPieces() },
    // A Fleet of its own for each window, so a walk told twice starts from the same Job.
    behaves: (handle) => {
      const own = new FakeSessionsFleet([ruleRecord()], ruleThreads()).scenario(board).behaves!(handle);
      bindTeachFleet({ ask: () => undefined, kept: () => undefined }, handle);
      return { ...own, ...teachedMods };
    },
  };
}

export const teachRule: Scenario = build();
