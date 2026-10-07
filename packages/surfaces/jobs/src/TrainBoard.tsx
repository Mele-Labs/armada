// Overview for a Job whose members are Jobs — the board the Convoy artboard
// draws. The banner leads, the order is the left column and the one decision
// open on it is the right.
//
// It takes the destination the way `LandBoard` does: the parent's own run is
// the Workflow tab's, its plan the Plan tab's, its ledger the Record tab's.
// What Overview owes a person here is what the set needs from them.

import { useRef } from "react";
import { Button, JobMembers, MemberDecision } from "@armada/components";

import { JobLead } from "./JobLead";

import type { MembersBoard } from "./members";

export type TrainBoardProps = {
  read: MembersBoard;
  onSaid: (sentence: string) => void;
  onCopied: (value: string) => void;
};

export function TrainBoard({ read, onSaid, onCopied }: TrainBoardProps) {
  const decision = useRef<HTMLDivElement>(null);

  return (
    <div className="armada-train">
      {/* The thing waiting on a person, before anything they would have to
          look for it among. `JobLead` is the ordinary Job's banner too. */}
      <JobLead
        {...read.lead}
        act={
          read.decision === undefined ? undefined : (
            <Button
              onClick={() => {
                decision.current?.scrollIntoView({ block: "nearest" });
                decision.current?.querySelector("button")?.focus();
              }}
            >
              Answer it
            </Button>
          )
        }
      />

      <div className="armada-train__columns" data-alone={read.decision === undefined || undefined}>
        <JobMembers {...read.train} onSaid={onSaid} onCopied={onCopied} />
        {read.decision === undefined ? null : (
          <div className="armada-train__decision" ref={decision}>
            <MemberDecision {...read.decision} />
          </div>
        )}
      </div>
    </div>
  );
}
