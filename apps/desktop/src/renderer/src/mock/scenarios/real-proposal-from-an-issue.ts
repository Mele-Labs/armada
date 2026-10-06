// A Job at its gate whose criteria were read from an issue that has moved
// since, as Fleet serves it at 23.8, with its repository's branches (#1765).

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { proposalFromAnIssue } from "@armada/jobs/fake";

export const s090ProposalFromAnIssue: Scenario = holding("real/proposal-from-an-issue", proposalFromAnIssue().name, [proposalFromAnIssue()], {
  opens: proposalFromAnIssue().job.id,
});
