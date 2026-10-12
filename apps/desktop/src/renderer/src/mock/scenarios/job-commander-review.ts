// A Job at its review gate, with the Commander over its canvas. The approval the lead held is a card in
// the Now panel, drawn as the Cockpit's call cards are. Mock data only. The walk `job-commander-review` plays it.

import { review } from "@armada/jobs/fixtures/build/index";

import { commandedBy } from "../commander-fleet";
import { asRow, holding } from "../holding";
import type { Scenario } from "../moment";

const fixture = asRow(review(), 15, "commander-review", "Retry a short read once");

const idle = commandedBy({ job: fixture.job.id, open: (c) => c.state("idle") });

export const s233JobCommanderReview: Scenario = {
  ...holding("job-commander-review", "A Job at its review gate: the approval is an ask in the Now panel", [fixture], { opens: fixture.job.id }),
  draft: { commanding: idle },
};
