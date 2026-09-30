// The proposal's screen, as the window mounts it: out of `App.tsx`, which is at
// the length the gate refuses, and beside `Composing.tsx` because the two are
// one press apart.
//
// **What dispatching lands on**, and why it is not the Job page: the owner's
// decision of 30 Sep 2026, *the wait is a destination*.

import { ProposalPage } from "@armada/components";
import type { ProposalAnswer } from "@armada/components";
import { PROPOSAL_IS_SLOW, watchOf } from "@armada/screens";
import { Boundary } from "@armada/shell";

import type { BridgeState } from "../../shared/bridge";
import type { useCommands } from "./commands";
import { useDrafted } from "./drafted";
import { asTheBoardSays } from "./proposing";
import type { ProposalRun } from "./proposing";

export function Proposal({
  run,
  state,
  commands,
  now,
  onOpenJob,
  onEdit,
  onAnother,
  onCopied,
}: {
  /** The proposal on screen. The window holds it; `proposing.ts` is the fold. */
  run: ProposalRun;
  state: BridgeState;
  commands: ReturnType<typeof useCommands>;
  /** The window's one clock, so no two elapsed figures on screen disagree. */
  now: number;
  /** Open a Job the request became, and stop showing the proposal. */
  onOpenJob: (jobId: string) => void;
  /** Back to the composer, which still holds the words. */
  onEdit: () => void;
  /** Back to the composer, empty. */
  onAnother: () => void;
  onCopied: (value: string) => void;
}) {
  // A moment being replayed carries what was sent as the composer's own draft;
  // a window reopened mid-call carries nothing, and the screen draws no quote.
  const drafted = useDrafted();
  const request = run.sent ?? drafted.prompt;
  const watch = watchOf(state.proposing, now);
  // The wait is Fleet's reading of the call, and the answer is the board's
  // reading of the Jobs. Joined here and nowhere else.
  const shown: ProposalAnswer =
    run.at.at === "reading" && watch !== null
      ? { at: "reading", watch }
      : asTheBoardSays(run.at, state.jobs);

  return (
    <div className="armada-screen__pane">
      <Boundary region="the proposal" bridge={state.bridge} onCopied={onCopied}>
        <ProposalPage
          {...(request === undefined ? {} : { request })}
          proposal={shown}
          slowAfterMs={PROPOSAL_IS_SLOW}
          // Kills the call rather than leaving the screen: a wait abandoned
          // leaves the proposer spending with nobody left to read it.
          onStop={() => void commands.stopProposal()}
          onOpen={onOpenJob}
          // The same command the detail's own gate calls, so a second approval
          // is refused by the one guard rather than by two.
          onApprove={(jobId) => void commands.approve(jobId)}
          approving={state.approving}
          onEdit={onEdit}
          onAnother={onAnother}
          onCopied={onCopied}
        />
      </Boundary>
    </div>
  );
}
