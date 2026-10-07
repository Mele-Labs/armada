// A step the gaming check holds, with its two answers — the one block every
// surface draws for it. #1079, #1672.
//
// **Under Overview's lead and in the Workflow step panel** (owner, 2 Oct
// 2026), the arrangement `judge-asked.tsx` took for a Judge's refusal: one
// block, the same handlers, so the two cannot offer different acts at a flag.
// It had no host since the Overview reframe of 29 Sep 2026 took the step
// inspector that drew it, and a held Job sat until it was killed.
//
// **Both answers are acts that already exist.** Carry on is the override, which
// Fleet takes with a blank reason on a gaming flag. Send it back is a redirect
// where the Drone still holds its session, which is nearly every flag, and the
// step restart where it has gone, which briefs the next Drone with the flag.

import { HeldFlag, type HeldFinding, type HeldFlagAnswer } from "@armada/components";
import type { Diff, JobDetail as JobWhole, JobSummary, StepDetail } from "@armada/protocol";

import { flagSaid, flagsOf, heldByAFlag, hunkFor, sentBackWords } from "./gaming";
import { openKept, type Opens } from "./phases";
import type { ActingAct } from "./pending";
import { onwards, recourseOf } from "./recovery";

export type GamingHeldProps = {
  job: JobSummary;
  whole: JobWhole | null;
  /** The step being read. The block is drawn only where a flag holds it. */
  step: StepDetail | undefined;
  /** This Job's patch, as the screen already holds it. The hunks come out of it. */
  diff: Diff;
  /** How a flag's brief opens. */
  opens: Opens;
  /** Every control is refused while what is shown is not live. */
  stale: boolean;
  acting: boolean;
  /** Which act, where `acting` is true — `override_verdict`, or the one Send it back sends. #1117. */
  actingAct?: ActingAct | undefined;
  /** Carry on: the override, with the person's reason or none. */
  onOverrule: (jobId: string, reason: string) => void;
  /** Send it back where the Drone has gone: the restart, with the note. */
  onSendBack: (jobId: string, note?: string) => void;
  /** Send it back where the Drone still holds its session: the redirect, with the flag and the note. */
  onRedirect: (jobId: string, instruction: string) => void;
  /**
   * Drawn under Overview's lead, which already says what each flag caught, so
   * the card does not say it again. The step panel has no lead over it.
   */
  underTheLead?: boolean;
};

/** What *Send it back* does where the Drone still holds its session. */
const REDIRECTS =
  "Sends the flag back to the drone still on this step, with your note if you write one, and it " +
  "works the step again in the same session.";

/** What it does where the Drone has gone. `restart_step` briefs the next Drone with the flag. */
const RESTARTS =
  "Restarts the step with a fresh drone. Its brief carries the flag, and your note if you write one.";

/** Where Fleet offers neither. */
const NEITHER = "Fleet offers neither a redirect nor a restart on this step.";

/** Where Fleet offers no override on this step. */
const NO_OVERRIDE = "Fleet offers no override on this step.";

const STALE = "This Job is not live, so nothing can be sent.";

/**
 * What the flag caught, the lines, what it asked, and both answers. Nothing
 * where no flag holds the step.
 */
export function GamingHeld({
  job,
  whole,
  step,
  diff,
  opens,
  stale,
  acting,
  actingAct,
  onOverrule,
  onSendBack,
  onRedirect,
  underTheLead = false,
}: GamingHeldProps) {
  if (step === undefined || !heldByAFlag(whole, step)) return null;
  const recourse = recourseOf(job, whole);
  const overrule = recourse.overrule?.trigger === "evidence_suspect" ? recourse.overrule : undefined;
  const flags = flagsOf(step).held;
  const findings: HeldFinding[] = flags.map((flag) => {
    const located = hunkFor(flag, diff, job.id);
    return {
      label: flagSaid(flag),
      ...(underTheLead ? {} : { headline: flagSaid(flag) }),
      // The lines where the patch holds them, and what the check quoted where
      // it does not — never a hunk near the one it meant.
      ...(located === undefined
        ? { cited: flag.cited, ...(flag.at === undefined ? {} : { at: flag.at }) }
        : { hunk: { path: located.file, lines: located.lines } }),
      ...(flag.asked === undefined ? {} : { asked: flag.asked }),
      ...(flag.brief_path === undefined ? {} : { brief: flag.brief_path }),
    };
  });
  // **Fleet's answer, never guessed**: a Drone still holding its session takes
  // a redirect, and one that has gone takes a restart.
  const sendsBy = recourse.act;
  // Which of the two answers is out, named directly — `override_verdict` and
  // `sendsBy` are two distinct acts, so nothing here has to remember which
  // control sent it. #1117.
  const pending: HeldFlagAnswer | undefined = !acting
    ? undefined
    : actingAct === "override_verdict"
      ? "carryOn"
      : sendsBy !== undefined && actingAct === sendsBy
        ? "sendBack"
        : undefined;
  return (
    <HeldFlag
      findings={findings}
      onOpenBrief={(brief) => openKept(opens, { kept: brief, what: "brief" })}
      carryOn={{
        consequence: overrule === undefined ? "Overrules the flag." : `Overrules the flag. ${onwards(overrule)}`,
        ...(overrule === undefined ? { withheld: NO_OVERRIDE } : {}),
        onCarryOn: (reason) => onOverrule(job.id, reason),
      }}
      sendBack={{
        consequence: sendsBy === "redirect" ? REDIRECTS : RESTARTS,
        ...(sendsBy === undefined ? { withheld: NEITHER } : {}),
        onSendBack: (note) =>
          sendsBy === "redirect" ? onRedirect(job.id, sentBackWords(flags, note)) : onSendBack(job.id, note),
      }}
      disabled={stale || acting}
      disabledNote={stale ? STALE : undefined}
      pending={pending}
    />
  );
}
