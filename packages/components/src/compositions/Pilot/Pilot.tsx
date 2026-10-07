import { createContext, useContext, useState } from "react";
import { SquareTerminal, Terminal } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Dialog } from "../../primitives/Dialog/Dialog";
import { Radio, RadioGroup } from "../../primitives/Radio/Radio";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { OwnerChip } from "../OwnerChip/OwnerChip";

/**
 * Piloting a Job: a person takes its worktree and works it with an agent in a
 * Session, then ends the pilot through one of three exits.
 *
 * **The three exits are drawn in both places the owner named**: on the
 * piloting Session's ledger row for the Job and on Job detail. This module is
 * the one drawing of them, so the two cannot disagree. It differs from
 * "Open in a Session" in what it does to the Drone: that one leaves it
 * working, this one stops it.
 *
 * A host provides `PilotAct`; where none does, nothing here is drawn.
 */
export type PilotOutcome = "take_over" | "restart_step";
export type PilotExit = "submit" | "attest" | "supersede";

export type PilotValue = {
  /** Opens the confirmation for a Job. */
  ask: (jobId: string) => void;
  /** The Session piloting a Job, while one is. */
  pilotedBy: (jobId: string) => { id: string; title: string; number: number } | undefined;
  exit: (jobId: string, exit: PilotExit) => void;
  open: (sessionId: string) => void;
};

export const PilotAct = createContext<PilotValue | null>(null);

export function usePilot(): PilotValue | null {
  return useContext(PilotAct);
}

/** Whether a Job in this status can be piloted: running, or escalated with no Drone left. */
export const pilotable = (status: string): boolean => status === "running" || status === "escalated";

/** The act, in a Job's header. Primary on an escalated Job, secondary on a running one: one slot, two fills. */
export function PilotButton({ jobId, status }: { jobId: string; status: string }) {
  const pilot = usePilot();
  if (pilot === null || !pilotable(status)) return null;
  return (
    <Tooltip label={status === "escalated" ? "Take this Job's worktree. Nothing is running on it" : "Stops the Drone and gives you its worktree"}>
      <Button variant={status === "escalated" ? "primary" : "secondary"} onClick={() => pilot.ask(jobId)}>
        <Terminal size={16} strokeWidth={2} aria-hidden />
        Pilot
      </Button>
    </Tooltip>
  );
}

/** Which Session is piloting a Job, as a chip that opens its owner card on hover. */
/**
 * `compact` is the mark alone, for a Board row where a title does not fit: the Session is
 * its tooltip and its card.
 */
export function PilotedBy({ jobId, compact = false }: { jobId: string; compact?: boolean }) {
  const pilot = usePilot();
  const by = pilot?.pilotedBy(jobId);
  if (pilot === null || by === undefined) return null;
  const chip = compact ? (
    <Tooltip label={`Piloted in ${by.title}`}>
      <span className="armada-pilot-chip" data-compact role="img" aria-label={`Piloted in ${by.title}`}>
        <SquareTerminal size={12} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  ) : (
    <span className="armada-pilot-chip">
      <SquareTerminal size={12} strokeWidth={2} aria-hidden />
      {by.title}
    </span>
  );
  return (
    <OwnerChip chip={{ kind: "job", id: jobId, number: by.number }} plain={chip}>
      {chip}
    </OwnerChip>
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
    <div className="armada-pilot-exits" role="group" aria-label="Ways out of the pilot" data-compact={compact || undefined}>
      {EXITS.map(({ exit, label, says }) => (
        <Tooltip key={exit} label={says}>
          <Button size="sm" variant={exit === "submit" ? "primary" : "secondary"} onClick={() => pilot.exit(jobId, exit)}>
            {label}
          </Button>
        </Tooltip>
      ))}
    </div>
  );
}

/**
 * The confirmation: what each outcome does to the Drone and to the worktree.
 * **Assist is drawn and off**, so the set does not change shape when it ships.
 */
export function PilotConfirm({ open, title, onConfirm, onCancel }: { open: boolean; title: string; onConfirm: (outcome: PilotOutcome) => void; onCancel: () => void }) {
  const [outcome, setOutcome] = useState<PilotOutcome>("take_over");
  return (
    <Dialog open={open} title="Pilot this Job?" tone="neutral" confirmLabel="Pilot" cancelLabel="Cancel" onConfirm={() => onConfirm(outcome)} onCancel={onCancel}>
      <p className="armada-pilot-confirm__job">{title}</p>
      <RadioGroup label="What happens to it">
        <Radio name="pilot-outcome" checked={outcome === "take_over"} onChange={() => setOutcome("take_over")}>
          Take Over
        </Radio>
        <p className="armada-pilot-confirm__says">The Drone stops. The worktree is yours for good.</p>
        <Radio name="pilot-outcome" checked={outcome === "restart_step"} onChange={() => setOutcome("restart_step")}>
          Restart Step
        </Radio>
        <p className="armada-pilot-confirm__says">The Drone stops. The worktree goes to a new Drone at the step that failed, afterwards.</p>
        <Radio name="pilot-outcome" checked={false} disabled onChange={() => undefined}>
          Assist
        </Radio>
        <p className="armada-pilot-confirm__says">Coming soon.</p>
      </RadioGroup>
    </Dialog>
  );
}
