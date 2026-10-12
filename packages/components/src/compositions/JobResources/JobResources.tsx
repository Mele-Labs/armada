// Pulse — what one Job is costing the machine right now: what is running, what
// it is spending, the processes, the checkouts and the logs.
//
// Not a debug panel. The figures come first; a sentence only where Fleet
// cannot be asked. `docs/contracts/design-system.md` → Fleet panel, for the
// figure rows; #1538 for the board.

import { RotateCw } from "lucide-react";
import { useState } from "react";

import type { Artifact, JobExamined, Look } from "@armada/protocol";
import { Button } from "../../primitives/Button/Button";
import { SkeletonText } from "../../primitives/Skeleton/Skeleton";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { DestinationCard } from "../DestinationCard/DestinationCard";
import { FigureList, type Figure } from "../FigureList/FigureList";
import { GuideMark } from "../GuideMark/GuideMark";
import { GUIDE_LOOK, GUIDE_PROCESSES, GUIDE_PULSE, GUIDE_WORKTREE_SIZE } from "../../guides";
import {
  ago,
  ANY,
  CPU_USAGE,
  KillHold,
  LogMember,
  Logs,
  MEMORY_USAGE,
  nothingRunningIsAFault,
  Processes,
  processesTotal,
  TOTAL_SIZE_ON_DISK,
  Worktrees,
  worktreesTotal,
} from "./JobResources.lists";
import type { PulseLogRow, PulseProcessRow, PulseReading } from "./JobResources.lists";

export * from "./JobResources.lists";

/**
 * Why the panel offers nothing to press.
 *
 * `no_answer` — Fleet is not on the other end. Bridge holds no connection, or
 * the request itself could not be sent.
 *
 * `unreadable` — Fleet answered and Bridge could not read what came back. Fleet
 * is demonstrably up, which is exactly why the act still goes: the two do not
 * agree about the route, and the same request down the same route meets the
 * same disagreement.
 */
export type NothingToAsk = "no_answer" | "unreadable";

export type JobResourcesProps = {
  /**
   * The reading, or `null` where none has arrived.
   *
   * **`null` is not empty.** A read that has not answered and a job that holds
   * nothing are different things, and `note` is what says which.
   */
  reading: PulseReading | null;
  /**
   * What is running and what it is spending, as label and figure rows.
   *
   * **Above the reading and not inside it.** These come off the Job Fleet
   * answered with, so they are there whether or not anybody has looked at the
   * machine, and the instant under the tables does not qualify them.
   */
  figures?: Figure[];
  /**
   * The Job those figures come off has not been read yet. **Bars stand where
   * they will land**, because `Checks running 0` before the read is a count
   * nobody took. `figures` is not drawn while this holds.
   */
  figuresReading?: boolean;
  /** Why there is no reading, where there is none. */
  note?: string;
  /** How old the reading is, as a phrase — `4s`. Formatted by the caller. */
  age?: string;
  /** What the last look found, or `null` where nobody has pressed. */
  examined: JobExamined | null;
  /** A look already out. A second press does not send a second act. */
  looking?: boolean;
  /** Why the last look failed, where it did. Drawn instead of a finding. */
  lookFailed?: string;
  /**
   * There is nothing to ask, and which of the two reasons it is. **The act is
   * not drawn at all**, and the panel says why rather than leaving a dead
   * control on screen.
   *
   * **Two readings and not one**, because the fixes are opposite: `no_answer`
   * is a Fleet that may only need starting, `unreadable` one that would come
   * back the same. The caller decides it from what the failure was, never from
   * the fact that something failed.
   */
  nothingToAsk?: NothingToAsk;
  onExamine: () => void;
  /**
   * Open a worktree or a log. **Absent draws no `Open` on any row**, so a
   * surface that cannot open files does not offer to.
   */
  onOpen?: (what: Artifact) => void;
  /**
   * Open the Job's worktree in a terminal, beside the worktree row's `Open`. **Absent draws no
   * control**, `onOpen`'s rule.
   */
  onOpenTerminal?: () => void;
  /**
   * Read a log in a panel, by pressing its row. **Absent draws no row as
   * pressable**, so a surface with no panel to open does not offer one.
   */
  onViewLog?: (log: PulseLogRow) => void;
  /** The path of the log whose panel is open, so its row says it is expanded. */
  viewingLog?: string;
  /**
   * Kill one process, once its row's kill has been held. **The host confirms**
   * — this only says which process was asked for. Absent draws no kill on any
   * row.
   */
  onKillProcess?: (process: PulseProcessRow) => void;
  /**
   * Kill every process the Job holds, once the head's kill has been held. The
   * host confirms. Absent draws no control, and so does a list with nothing in
   * it.
   */
  onKillAll?: () => void;
};

/**
 * **Nothing running is drawn as loudly as a failure.** A job that reads
 * running and holds no process is the state that took a terminal to
 * establish, and an empty list under a heading is how it went unnoticed — so
 * each absence is a sentence in its own words rather than a table with no rows.
 */
export function JobResources({
  reading,
  figures = [],
  figuresReading = false,
  note,
  age,
  examined,
  looking = false,
  lookFailed,
  nothingToAsk,
  onExamine,
  onOpen,
  onOpenTerminal,
  onViewLog,
  viewingLog,
  onKillProcess,
  onKillAll,
}: JobResourcesProps) {
  const [member, setMember] = useState(ANY);
  return (
    <section className="armada-holds">
      <DestinationCard
        label={STATS}
        guide={GUIDE_PULSE}
        trailing={
          <>
            <Headline lookFailed={lookFailed} nothingToAsk={nothingToAsk} />
            {reading === null ? null : (
              <span className="armada-holds__read-at">{readAt(reading, age)}</span>
            )}
            {/* No act where there is nothing to ask. **Absent rather than
                disabled**: a greyed control still says an act exists here and
                puts the reason on a person to work out.

                **`rotate-cw` alone, `Refresh` in its tooltip** — the owner's
                word on 28 Sep, and his call for an icon button on 29 Sep. */}
            {nothingToAsk !== undefined ? null : (
              <span className="armada-holds__act">
                <Tooltip label="Refresh">
                  <Button
                    variant="ghost"
                    size="sm"
                    iconOnly
                    aria-label={looking ? "Refreshing" : "Refresh"}
                    onClick={onExamine}
                    disabled={looking}
                  >
                    <RotateCw size={16} strokeWidth={2} aria-hidden="true" />
                  </Button>
                </Tooltip>
                <GuideMark guide={GUIDE_LOOK} />
              </span>
            )}
          </>
        }
      >
        {figuresReading ? (
          <SkeletonText />
        ) : figures.length === 0 ? null : (
          <FigureList figures={figures} column="strip" />
        )}
        {examined === null ? null : <Looks looks={examined.looks} />}
        {reading === null && (nothingToAsk !== undefined || note !== undefined) ? (
          <p className="armada-holds__note">
            {nothingToAsk === undefined ? note : INSTEAD[nothingToAsk]}
          </p>
        ) : null}
      </DestinationCard>

      {reading === null ? null : (
        <>
          {/* **A card for every list, including an empty one.** A card that
              disappeared when it held nothing would make "no worktree on
              disk" and "this build does not draw worktrees" the same screen.
              An empty one keeps its head and draws nothing under it. */}
          <div className="armada-holds__pair">
            <DestinationCard
              label="Processes"
              guide={GUIDE_PROCESSES}
              trailing={
                <ProcessesHead
                  processes={reading.processes}
                  {...(onKillAll === undefined ? {} : { onKillAll })}
                />
              }
            >
              {quietlyEmpty(reading, examined) ? null : (
                <Processes
                  reading={reading}
                  examined={examined}
                  {...(onKillProcess === undefined ? {} : { onKill: onKillProcess })}
                />
              )}
            </DestinationCard>
            <DestinationCard
              label="Worktrees"
              guide={GUIDE_WORKTREE_SIZE}
              trailing={
                worktreesTotal(reading.worktrees) === "" ? null : (
                  <Tooltip label={TOTAL_SIZE_ON_DISK} asChild>
                    <span className="armada-holds__count">{worktreesTotal(reading.worktrees)}</span>
                  </Tooltip>
                )
              }
            >
              {reading.worktrees.length === 0 ? null : (
                <Worktrees
                  worktrees={reading.worktrees}
                  {...(onOpen === undefined ? {} : { onOpen })}
                  {...(onOpenTerminal === undefined ? {} : { onOpenTerminal })}
                />
              )}
            </DestinationCard>
          </div>
          <DestinationCard
            label="Job logs"
            trailing={<LogMember logs={reading.logs} member={member} onMember={setMember} />}
          >
            {reading.logs.length === 0 ? null : (
              <Logs
                logs={reading.logs}
                member={member}
                {...(onOpen === undefined ? {} : { onOpen })}
                {...(onViewLog === undefined ? {} : { onView: onViewLog })}
                {...(viewingLog === undefined ? {} : { viewing: viewingLog })}
              />
            )}
          </DestinationCard>
        </>
      )}
    </section>
  );
}

/** No process held, and nothing wrong with that: the Processes card holds nothing. */
function quietlyEmpty(reading: PulseReading, examined: JobExamined | null): boolean {
  return reading.processes.length === 0 && reading.held === "none" && !nothingRunningIsAFault(reading.held, examined);
}

/**
 * The Processes card's head: CPU and memory across the list, each named by its
 * tooltip, and the kill that reaches every row.
 *
 * **Nothing where there are no processes** — no figures and no kill, since
 * there is nothing to total and nothing to end. The card's sentence says why.
 */
function ProcessesHead({
  processes,
  onKillAll,
}: {
  processes: PulseProcessRow[];
  onKillAll?: () => void;
}) {
  const total = processesTotal(processes);
  if (total === null) return null;
  return (
    <>
      <span className="armada-holds__count">
        <Tooltip label={CPU_USAGE} asChild>
          <span>{total.cpu}</span>
        </Tooltip>
        {" · "}
        <Tooltip label={MEMORY_USAGE} asChild>
          <span>{total.memory}</span>
        </Tooltip>
      </span>
      {onKillAll === undefined ? null : (
        <KillHold label={HOLD_TO_KILL_ALL} asks={KILL_ALL} onKill={onKillAll} />
      )}
    </>
  );
}

/** The head's kill, while held and where a press asks instead. */
const HOLD_TO_KILL_ALL = "Hold to kill all";
const KILL_ALL = "Kill all processes";

/** The first card's name. The board's word for the figures under it. */
const STATS = "Stats";

/** When the figures were true — `Updated 4s ago`, and nothing beside it. */
function readAt(reading: PulseReading, age?: string): string {
  return age === undefined ? `Updated at ${reading.readAt}` : `Updated ${ago(age)}`;
}

/**
 * A sentence only where something failed: Fleet cannot be asked, or the look
 * that was asked for did not come back.
 *
 * **Nothing otherwise.** A line saying the job had not been looked at sat
 * beside `Updated 4s ago` and read as a contradiction (owner, 29 Sep: "We dont
 * need to say anything"). A look's result is the list under the head.
 */
function Headline({
  lookFailed,
  nothingToAsk,
}: {
  lookFailed?: string;
  nothingToAsk?: NothingToAsk;
}) {
  // Ahead of a failed look: this is the reason there can be no look at all.
  if (nothingToAsk !== undefined) {
    return (
      <p className="armada-holds__verdict" data-degraded>
        <span className="armada-holds__dot" aria-hidden="true" />
        {SILENT[nothingToAsk]}
      </p>
    );
  }
  if (lookFailed !== undefined) {
    return (
      <p className="armada-holds__verdict" data-found="not_working">
        <span className="armada-holds__dot" aria-hidden="true" />
        {lookFailed}
      </p>
    );
  }
  return null;
}

/**
 * The headline, when there is nothing to ask.
 *
 * **Both name Fleet as the subject**: a job that holds nothing is a real
 * answer, and a sentence that did not say which would report a silent seam as
 * a silent job. **They must not read as the same message twice**, because the
 * fixes point in opposite directions.
 */
const SILENT: Record<NothingToAsk, string> = {
  no_answer: "Fleet is not answering, so there is nothing to ask.",
  unreadable: "Fleet answered, and Bridge could not read the answer.",
};

/**
 * What stands in for the reading, and neither of them is a reading.
 *
 * **`no_answer` points at the status bar and `unreadable` must not** — the bar
 * says "Fleet running" on the second, which would read as the two disagreeing.
 * So the sentence carries its own next step, and names the wrong move, because
 * a restart is the one somebody reaches for.
 */
const INSTEAD: Record<NothingToAsk, string> = {
  no_answer:
    "Nothing here is a reading of this job. The status bar names which Fleet state this is and what to do.",
  unreadable:
    "Nothing here is a reading of this job. This build of Bridge and this Fleet do not agree about the route, so restarting Fleet changes nothing. They ship as a pair, and rebuilding both is what settles it.",
};

/** What each look asked, in the order a person reads them. */
const ASKED: Record<Look["asked"], string> = {
  process: "The process",
  worktree: "The worktree",
  writing: "What was written",
  span: "Where the job is",
  silence: "The liveness watch",
  repeating: "The last two attempts",
  scope_drift: "Work outside the declared scope",
};

/** Every look, with the ones that could not tell marked as such. */
function Looks({ looks }: { looks: Look[] }) {
  return (
    <ul className="armada-holds__looks">
      {looks.map((look) => (
        <li key={look.asked} className="armada-holds__look" data-found={look.found}>
          <span className="armada-holds__asked">{ASKED[look.asked] ?? look.asked}</span>
          <span className="armada-holds__said">{look.said}</span>
          {look.fields === undefined || look.fields.length === 0 ? null : (
            <span className="armada-holds__fields">
              {look.fields.map((field) => (
                <span key={field.name} className="armada-holds__field">
                  <span className="armada-holds__field-name">{field.name}</span>
                  <span className="armada-holds__mono">{field.value}</span>
                </span>
              ))}
            </span>
          )}
        </li>
      ))}
    </ul>
  );
}

