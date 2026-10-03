// What a person has moved on a Job's proposal, held by the screen until the
// approval press sends it (`approve_dispatch`, since 23.8, #1641).
//
// **Two sources, one shape.** A mock moment's draft carries a proposal of its
// own, which `ProposalTab` draws; a real Job's is read off `JobDetail`, which
// Overview's approval panel draws under the lead. Either way the press sends
// what moved since it was read, and nothing else.

import { useEffect, useMemo, useState } from "react";
import type { ApproveDispatch, Branches, JobDetail as JobWhole, WorkflowSummary } from "@armada/protocol";

import { branchesFrom } from "./draft/branches";
import type { BranchView } from "./draft/branches";
import type { JobDraft } from "./draft/held";
import { approvalOf, proposalEditsOf, proposalEditsOfWhole } from "./tab-proposal-read";
import type { ProposalEdits } from "./tab-proposal-read";

export type ApprovalHeld = {
  /** What the proposal reads as now, a person's moves included. Absent where there is none. */
  edits: ProposalEdits | undefined;
  onEdits: (edits: ProposalEdits) => void;
  /** The moment carries a proposal of its own, so `ProposalTab` takes Overview's place. */
  drafted: boolean;
  /** A real Job waiting at its approval gate, whose panel reads under the lead. */
  atGate: boolean;
  /** What froze at approval, for Settings. Absent before anybody approved. */
  frozen: ProposalEdits | undefined;
  /** The body the press sends: what moved, or `undefined` where nothing did. */
  approval: () => ApproveDispatch | undefined;
  /** The repository's branches, read once the proposal is open. `null` before. */
  branches: readonly BranchView[] | null;
};

export function useApproval({
  draft,
  whole,
  workflows,
  onListBranches,
}: {
  draft: JobDraft | undefined;
  whole: JobWhole | null;
  workflows: readonly WorkflowSummary[];
  onListBranches: ((manifestId: string) => Promise<Branches | null>) | undefined;
}): ApprovalHeld {
  // The moment's own, read once: a draft is the mock's and never moves under a person.
  const [drafted] = useState(() => proposalEditsOf(draft));
  const [moved, setMoved] = useState<ProposalEdits | undefined>(undefined);
  const fromWhole = useMemo(() => (whole === null ? undefined : proposalEditsOfWhole(whole, null)), [whole]);
  const atGate = whole?.job.status === "awaiting_approval" && whole.approved_at === undefined;
  // Approved on the wire outranks the draft: the press is what moved it.
  const approved = whole?.approved_at !== undefined;
  const before = approved ? undefined : (drafted ?? (atGate ? fromWhole : undefined));
  const open = before !== undefined && before.proposal.approved_at === undefined;

  const [branches, setBranches] = useState<readonly BranchView[] | null>(null);
  const manifestId = whole?.job.owner_manifest_id;
  useEffect(() => {
    if (!open || manifestId === undefined || onListBranches === undefined) return;
    let current = true;
    void onListBranches(manifestId).then((answer) => {
      if (current && answer !== null) setBranches(branchesFrom(answer));
    });
    return () => {
      current = false;
    };
  }, [open, manifestId, onListBranches]);

  const edits = before === undefined ? undefined : (moved ?? before);
  return {
    edits,
    onEdits: setMoved,
    drafted: drafted !== undefined && !approved,
    atGate: atGate && drafted === undefined,
    frozen: approved ? fromWhole : drafted?.proposal.approved_at === undefined ? undefined : drafted,
    approval: () => (edits === undefined || before === undefined ? undefined : approvalOf(edits, before, workflows)),
    branches,
  };
}
