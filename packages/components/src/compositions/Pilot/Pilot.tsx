import { createContext, useContext } from "react";
import { Check } from "lucide-react";

import { Alert } from "../../primitives/Alert/Alert";
import { Button } from "../../primitives/Button/Button";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * A Job a person is piloting, ended through one of three exits.
 *
 * **The three exits are drawn in both places the owner named**: on a Session's
 * ledger row for the Job and on Job detail. This module is the one drawing of
 * them, so the two cannot disagree.
 *
 * A host provides `PilotAct`; where none does, nothing here is drawn.
 */
export type PilotExit = "submit" | "attest" | "supersede";

export type PilotValue = {
  exit: (jobId: string, exit: PilotExit) => void;
  /** What Fleet refused the last act in words, until the next one. Drawn under the exits. */
  said?: string | undefined;
};

export const PilotAct = createContext<PilotValue | null>(null);

export function usePilot(): PilotValue | null {
  return useContext(PilotAct);
}

/**
 * A Job a person closed on their word: its gates did not run. **Drawn apart from a Job that passed**,
 * which is the one thing an attestation must never read as. `compact` is the mark alone, for a Board
 * row; `note` is what they said, in the tooltip.
 */
export function AttestedMark({ note, compact = false }: { note?: string | undefined; compact?: boolean }) {
  const said = note === undefined || note === "" ? "Closed on a person's word. Its gates did not run" : `Closed on a person's word: ${note}. Its gates did not run`;
  return (
    <Tooltip label={said}>
      <span className="armada-pilot-chip" data-compact={compact || undefined} role="img" aria-label="Attested, not verified">
        <Check size={12} strokeWidth={2} aria-hidden />
        {compact ? null : "Attested"}
      </span>
    </Tooltip>
  );
}

const EXITS: { exit: PilotExit; label: string; says: string }[] = [
  { exit: "submit", label: "Submit for verification", says: "Runs this step's Checks and Judge on the worktree, as it would for a Drone" },
  { exit: "attest", label: "Attest complete", says: "Records the work as done by your word, not as verified" },
  { exit: "supersede", label: "Close as superseded", says: "Ends the Job: the work landed outside it" },
];

/** The three ways out of a pilot. */
export function PilotExits({ jobId, compact = false }: { jobId: string; compact?: boolean }) {
  const pilot = usePilot();
  if (pilot === null) return null;
  return (
    <>
      <div className="armada-pilot-exits" role="group" aria-label="Ways out of the pilot" data-compact={compact || undefined}>
        {EXITS.map(({ exit, label, says }) => (
          <Tooltip key={exit} label={says}>
            <Button size="sm" variant={exit === "submit" ? "primary" : "secondary"} onClick={() => pilot.exit(jobId, exit)}>
              {label}
            </Button>
          </Tooltip>
        ))}
      </div>
      {pilot.said === undefined || pilot.said === "" ? null : (
        <div className="armada-pilot-said">
          <Alert tone="escalated">{pilot.said}</Alert>
        </div>
      )}
    </>
  );
}
