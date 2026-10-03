import { CircleDot, Eye, Scale, ShieldEllipsis, type LucideIcon } from "lucide-react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * Which part of a running step it is in — a mark beside the step's own, named
 * by its tooltip and never by a phrase (owner's annotation of 3 Oct 2026,
 * `ouqa`: *"I had no idea it was running checks until i finally noticed that
 * the checks were updating in the panel when I opened it"*).
 *
 * **What is still working moves; a person's turn holds still.** A Drone, the
 * gate's Checks and the Judge are machines at work, so their marks breathe at
 * `--duration-pulse`, and stop under reduced motion. Waiting on you is nobody
 * working, so `eye` stands as the step's own waiting mark does.
 *
 * The glyphs are the registry's: `circle-dot` a Drone working, `shield-ellipsis`
 * and `scale`'s live usage proposed for this mark, `eye` the step's waiting
 * borrowing. `packages/icons/icons.toml`.
 */
export type StepPhase = "drone" | "checks" | "judge" | "waiting";

const GLYPH: Record<StepPhase, LucideIcon> = {
  drone: CircleDot,
  checks: ShieldEllipsis,
  judge: Scale,
  waiting: Eye,
};

export type StepPhaseMarkProps = {
  phase: StepPhase;
  /** What the mark says, on hover and to a screen reader — `Checks running`, `Drone on T4`. */
  label: string;
};

export function StepPhaseMark({ phase, label }: StepPhaseMarkProps) {
  const Icon = GLYPH[phase];
  return (
    <Tooltip asChild label={label}>
      <span className="armada-step-phase" data-phase={phase} role="img" aria-label={label}>
        <Icon size={12} strokeWidth={2} aria-hidden="true" />
      </span>
    </Tooltip>
  );
}
