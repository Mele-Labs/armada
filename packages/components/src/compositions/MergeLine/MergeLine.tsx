import { Fragment } from "react";
import { ArrowUpToLine, Ban, ChevronRight, ChevronUp, GitBranch, GitCommitHorizontal, Pause, type LucideIcon } from "lucide-react";

import { CHECK_OUTCOME, LAND_STATE } from "../../generated/vocabulary";
import { Alert, type AlertTone } from "../../primitives/Alert/Alert";
import { Badge } from "../../primitives/Badge/Badge";
import { Separator } from "../../primitives/Separator/Separator";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { GroupBoundary, type GroupBoundaryCheck, type GroupBoundaryCheckReads } from "../GroupBoundary/GroupBoundary";

/**
 * The merge line: the branches waiting to land on main through `armada land`,
 * in place order, then Recently landed and Sent back. An Overview panel.
 *
 * **A batch is drawn as one group, not as a "together with" string on every
 * member.** The branches a turn gates at once are consecutive in place order,
 * so they nest in one list with a bracket down their leading edge.
 *
 * **State is the mark alone**, from `land_state`, its word the tooltip. No
 * count beside the rows; an empty list draws nothing, heading included. With
 * all three empty the panel draws a picture and no words, the owner's one
 * exception to the empty-state rule, 2 Oct 2026.
 */
/** `preparing` is Bridge's own: a `gating` turn that has run no Check yet. */
export type MergeLineState =
  | "waiting"
  | "preparing"
  | "gating"
  | "merging"
  | "landed"
  | "red"
  | "conflict"
  | "stopped";

/**
 * Why a waiting branch is not in the turn that is running, where there is a reason. A branch with
 * none is queued behind, and draws nothing.
 *
 * - `member`: left out because it clashes with another member of the turn.
 * - `main`: left out because it clashes with main. Its own `conflict` outcome is a different mark.
 * - `kept`: kept its place after a red, and is next up.
 * - `late`: joined after the running turn began.
 */
export type MergeLineWaiting = "member" | "main" | "kept" | "late";

export type MergeLineEntry = {
  branch: string;
  /** 1-based place in line. Absent once the branch has left the line. */
  place?: number;
  /** The open pull request, where the branch has one. */
  pr?: {
    number: number;
    url: string;
    /** How it ended, as the Job's own pull request badge reads it. Absent is nothing known. */
    settled?: { status: string; icon: LucideIcon; label: string };
  };
  state: MergeLineState;
  /** Waiting only: why it is not in the running turn. Absent draws nothing. */
  why?: MergeLineWaiting;
  /** What the runner is doing to it now, in the runner's own words. Read while gating or merging. */
  doing?: string;
  /** The turn's batch, by key. Consecutive entries with one key gate together. */
  batch?: string;
  /** Landed: the merge commit, short. */
  merge?: string;
  /** Red: the Checks that failed. */
  failed?: readonly string[];
  /** Conflict: the files main did not merge into. */
  conflicts?: readonly string[];
  /** Gating, red and stopped: each Check the turn runs, as it stands. Drawn as the boundary's strip. */
  checks?: readonly MergeLineCheck[];
};

export type MergeLineCheck = {
  name: string;
  state: "waiting" | "running" | "passed" | "failed" | "timed_out";
};

/**
 * What the owner is told while a turn runs and one of its Checks has failed: the rerun failed
 * too, and the Check is green on main. **One per line, because a line runs one turn at a time**,
 * so it sits in one slot and each later reading replaces it rather than stacking a second.
 *
 * - `batch`: before the split names the one at fault, a heads-up to every branch in the turn.
 * - `branch`: the split named it, or the turn is a single branch.
 * - `main`: the same Check is red on main too. Nobody is blamed, and the turn holds.
 * - `sent`: the turn's verdict, the branch sent back.
 */
export type MergeLineNotice = {
  kind: "batch" | "branch" | "main" | "sent";
  /** The Check that failed. */
  check: string;
  /** The branch whose turn wrote the Check's log. Named on the notice only where a batch has more than one. */
  branch: string;
};

export type MergeLineProps = {
  /** The repository, where more than one line draws. Beside the heading. */
  name?: string;
  /** In place order. */
  line: readonly MergeLineEntry[];
  /** The turn's failed Check, drawn above the line. Absent is nothing to say, and nothing is drawn. */
  notice?: MergeLineNotice;
  /** Landed, newest first. */
  landed?: readonly MergeLineEntry[];
  /** Red, conflict or stopped and not back in line, newest first. */
  sentBack?: readonly MergeLineEntry[];
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Hands the pull request's address to whatever opens addresses on this machine. */
  onOpenPullRequest: (url: string) => void;
  /**
   * Open one Check's log, by the branch whose turn runs it. Absent, no Check is pressable. A
   * Check still waiting has no log, so it never is.
   */
  onOpenCheck?: (branch: string, check: string) => void;
  /** On the outer `<section>`, so a press elsewhere can scroll to it. */
  id?: string;
};

const HEADING = "Merge line";

/** The states the runner is working in. Their mark pulses. */
const LIVE: ReadonlySet<MergeLineState> = new Set(["preparing", "gating", "merging"]);

/** 12px at strokeWidth 2, the badge's own geometry. */
const MARK = 12;
const STROKE = 2;

/** The lists under the line, in the order they draw. */
const LEFT = [
  { heading: "Recently landed", pick: (props: MergeLineProps) => props.landed ?? [] },
  { heading: "Sent back", pick: (props: MergeLineProps) => props.sentBack ?? [] },
] as const;

export function MergeLine(props: MergeLineProps) {
  const { name, line, notice, open, onOpenChange, onOpenPullRequest, onOpenCheck, id } = props;
  const acts = { onOpenPullRequest, ...(onOpenCheck === undefined ? {} : { onOpenCheck }) };
  const named = name === undefined ? HEADING : `${HEADING}, ${name}`;
  const left = LEFT.map((one) => ({ heading: one.heading, entries: one.pick(props) })).filter(
    (one) => one.entries.length > 0,
  );
  return (
    <section className="armada-merge-line" id={id} aria-label={named}>
      <header className="armada-merge-line__head">
        <h2 className="armada-merge-line__heading">
          {HEADING}
          {name === undefined ? null : (
            <Tooltip label="Repository" asChild>
              <span className="armada-merge-line__repository">{name}</span>
            </Tooltip>
          )}
        </h2>
        <button
          type="button"
          className="armada-merge-line__fold"
          aria-expanded={open}
          aria-label={`${open ? "Collapse" : "Expand"} ${named}`}
          onClick={() => onOpenChange(!open)}
        >
          {open ? (
            <ChevronUp size={14} strokeWidth={2} aria-hidden />
          ) : (
            <ChevronRight size={14} strokeWidth={2} aria-hidden />
          )}
        </button>
      </header>
      {!open ? null : line.length === 0 && left.length === 0 && notice === undefined ? (
        <div className="armada-merge-line__empty">
          <EmptyLine />
        </div>
      ) : (
        <div className="armada-merge-line__body">
          {notice === undefined ? null : (
            <Notice notice={notice} batched={batched(line).some((run) => run.length > 1)} {...(onOpenCheck === undefined ? {} : { onOpenCheck })} />
          )}
          {line.length === 0 ? null : (
            <ol className="armada-merge-line__list" aria-label="In line">
              {batched(line).map((run) =>
                run.length === 1 ? (
                  <Entry key={run[0]!.branch} entry={run[0]!} {...acts} />
                ) : (
                  <li key={run[0]!.branch} className="armada-merge-line__batch">
                    <Tooltip label="Batch" decorative asChild>
                      <span className="armada-merge-line__bracket" aria-hidden />
                    </Tooltip>
                    <ol className="armada-merge-line__list" aria-label="Batch">
                      {run.map((entry) => (
                        <Entry key={entry.branch} entry={entry} {...acts} />
                      ))}
                    </ol>
                  </li>
                ),
              )}
            </ol>
          )}
          {left.map((one, n) => (
            <Fragment key={one.heading}>
              {line.length > 0 || n > 0 ? <Separator className="armada-merge-line__rule" /> : null}
              <h3 className="armada-merge-line__subheading">{one.heading}</h3>
              <ul className="armada-merge-line__list" aria-label={one.heading}>
                {one.entries.map((entry) => (
                  <Entry key={entry.branch} entry={entry} {...acts} />
                ))}
              </ul>
            </Fragment>
          ))}
        </div>
      )}
    </section>
  );
}

const NOTICE_TONE: Record<MergeLineNotice["kind"], AlertTone> = {
  batch: "caution",
  branch: "escalated",
  main: "neutral",
  sent: "escalated",
};

/**
 * The failed Check, in the Alert's own tones: caution for a heads-up to a whole batch, escalated
 * once a branch is named, neutral where main is red too and nobody is to blame. **Facts only, no
 * sentence**; the glyph and the Check's mark carry what the tone cannot, and caution takes none.
 */
function Notice({
  notice,
  batched,
  onOpenCheck,
}: {
  notice: MergeLineNotice;
  batched: boolean;
  onOpenCheck?: (branch: string, check: string) => void;
}) {
  const failed = CHECK_OUTCOME.failed;
  const Shield = failed?.icon ?? null;
  const glyph =
    notice.kind === "main" ? (
      <Tooltip label="Held" asChild>
        <span role="img" aria-label="Held">
          <Pause size={16} strokeWidth={2} aria-hidden />
        </span>
      </Tooltip>
    ) : notice.kind === "batch" || Shield === null ? undefined : (
      <Tooltip label="Check failed" asChild>
        <span role="img" aria-label="Check failed">
          <Shield size={16} strokeWidth={2} aria-hidden />
        </span>
      </Tooltip>
    );
  // A branch is named where the turn is a batch, and always once it has left the line.
  const named = notice.kind === "sent" || (notice.kind === "branch" && batched);
  return (
    <div className="armada-merge-line__notice">
      <Alert
        tone={NOTICE_TONE[notice.kind]}
        icon={glyph}
        title={
          <span className="mono">
            {notice.check}
            {notice.kind === "main" ? " red on main" : " failed"}
          </span>
        }
        {...(onOpenCheck === undefined
          ? {}
          : {
              action: (
                <button
                  type="button"
                  className="armada-alert__button"
                  onClick={() => onOpenCheck(notice.branch, notice.check)}
                >
                  Open log
                </button>
              ),
              actionOn: "title" as const,
            })}
      >
        <span className="armada-merge-line__facts">
          {named ? <span className="mono">{notice.branch}</span> : null}
          {notice.kind === "sent" ? <span>sent back</span> : null}
          {notice.kind === "batch" || notice.kind === "branch" ? (
            <>
              <span>rerun failed</span>
              <span>green on main</span>
            </>
          ) : null}
        </span>
      </Alert>
    </div>
  );
}

/**
 * A line nobody has been in: main, and a dashed branch with empty places on it
 * meeting main. Drawn in the border tokens, so it sits back from every row.
 * Its name is for a screen reader alone; nothing is written under it.
 */
function EmptyLine() {
  return (
    <svg className="armada-merge-line__picture" viewBox="0 0 96 40" role="img" aria-label="Empty">
      <path className="armada-merge-line__picture-main" d="M4 32 H92" />
      <path className="armada-merge-line__picture-branch" d="M4 10 H44 C58 10 58 32 72 32" />
      <circle className="armada-merge-line__picture-place" cx="16" cy="10" r="3.5" />
      <circle className="armada-merge-line__picture-place" cx="32" cy="10" r="3.5" />
      <circle className="armada-merge-line__picture-place" cx="72" cy="32" r="3.5" />
    </svg>
  );
}

/** The line cut into runs: a lone entry, or every consecutive entry of one batch. */
function batched(line: readonly MergeLineEntry[]): MergeLineEntry[][] {
  const runs: MergeLineEntry[][] = [];
  for (const entry of line) {
    const last = runs[runs.length - 1];
    if (entry.batch !== undefined && last !== undefined && last[0]!.batch === entry.batch) last.push(entry);
    else runs.push([entry]);
  }
  return runs;
}

function Entry({
  entry,
  onOpenPullRequest,
  onOpenCheck,
}: {
  entry: MergeLineEntry;
  onOpenPullRequest: (url: string) => void;
  onOpenCheck?: (branch: string, check: string) => void;
}) {
  const reading = LAND_STATE[entry.state];
  const Icon = reading?.icon ?? null;
  const said = reading?.verb ?? entry.state;
  return (
    <li className="armada-merge-line__row" aria-label={`${entry.branch}, ${said}`}>
      {entry.place === undefined ? (
        <span className="armada-merge-line__place" />
      ) : (
        <Tooltip label="Order to merge in" asChild>
          <span className="armada-merge-line__place mono">{entry.place}</span>
        </Tooltip>
      )}
      <Tooltip label={said} asChild>
        <span
          className="armada-merge-line__mark"
          data-pulsing={LIVE.has(entry.state) || undefined}
          data-state={entry.state}
          style={reading?.statusToken ? { color: `var(${reading.statusToken})` } : undefined}
          role="img"
          aria-label={said}
        >
          {Icon === null ? null : <Icon size={MARK} strokeWidth={STROKE} aria-hidden />}
        </span>
      </Tooltip>
      <span className="armada-merge-line__branch mono">{entry.branch}</span>
      <span className="armada-merge-line__pr">
        {entry.pr === undefined ? null : (
          <Tooltip label="Pull request">
            <a
              href={entry.pr.url}
              onClick={(event) => {
                event.preventDefault();
                onOpenPullRequest(entry.pr!.url);
              }}
            >
              #{entry.pr.number}
            </a>
          </Tooltip>
        )}
        {entry.pr?.settled === undefined ? null : (
          <Badge status={entry.pr.settled.status} icon={entry.pr.settled.icon}>
            {entry.pr.settled.label}
          </Badge>
        )}
      </span>
      <span className="armada-merge-line__detail">
        <Detail entry={entry} {...(onOpenCheck === undefined ? {} : { onOpenCheck })} />
      </span>
    </li>
  );
}

/** A waiting branch's reason as a registry glyph and the tooltip that names it. */
const WHY: Record<MergeLineWaiting, { Glyph: LucideIcon; says: string }> = {
  member: { Glyph: GitBranch, says: "Left out of this turn: clashes with another branch in it" },
  main: { Glyph: Ban, says: "Left out of this turn: clashes with main" },
  kept: { Glyph: ArrowUpToLine, says: "Kept its place after a red, next up" },
  late: { Glyph: GitCommitHorizontal, says: "Joined after the turn began" },
};

/** A Check of the turn on the boundary strip's own readings. A timeout is a failure there, and says so. */
const READS: Record<MergeLineCheck["state"], GroupBoundaryCheckReads> = {
  waiting: "not run",
  running: "running",
  passed: "passed",
  failed: "failed",
  timed_out: "failed",
};

/** A Check of the turn, opening its log where it has one: every one but a Check still waiting. */
function boundaryCheck(check: MergeLineCheck, onOpen: ((check: string) => void) | undefined): GroupBoundaryCheck {
  const timedOut = CHECK_OUTCOME.timed_out?.verb;
  return {
    name: check.name,
    reads: READS[check.state],
    ...(check.state === "timed_out" && typeof timedOut === "string" ? { result: timedOut } : {}),
    ...(onOpen === undefined || check.state === "waiting" ? {} : { onOpen: () => onOpen(check.name) }),
  };
}

/**
 * What the row says after its name: the runner's words and its Checks in a turn, the facts once off
 * the line. **The Checks are the plan's boundary strip**, not a second drawing of the same thing.
 */
function Detail({
  entry,
  onOpenCheck,
}: {
  entry: MergeLineEntry;
  onOpenCheck?: (branch: string, check: string) => void;
}) {
  const onOpen = onOpenCheck === undefined ? undefined : (check: string) => onOpenCheck(entry.branch, check);
  const strip =
    entry.checks === undefined || entry.checks.length === 0 ? null : (
      <GroupBoundary checks={entry.checks.map((check) => boundaryCheck(check, onOpen))} />
    );
  if (entry.state === "waiting" && entry.why !== undefined) {
    const { Glyph, says } = WHY[entry.why];
    return (
      <Tooltip label={says} asChild>
        <span className="armada-merge-line__why" role="img" aria-label={says}>
          <Glyph size={MARK} strokeWidth={STROKE} aria-hidden />
        </span>
      </Tooltip>
    );
  }
  if (LIVE.has(entry.state)) {
    return (
      <>
        {entry.doing === undefined ? null : <span>{entry.doing}</span>}
        {strip}
      </>
    );
  }
  if ((entry.state === "red" || entry.state === "stopped") && strip !== null) return strip;
  if (entry.state === "landed" && entry.merge !== undefined) {
    return (
      <Tooltip label="Merge commit">
        <span className="mono">{entry.merge}</span>
      </Tooltip>
    );
  }
  if (entry.state === "red" && entry.failed !== undefined) {
    const failed = CHECK_OUTCOME.failed;
    const Shield = failed?.icon ?? null;
    return (
      <>
        {entry.failed.map((name) => (
          <span key={name} className="armada-merge-line__check">
            {Shield === null ? null : (
              <Shield
                size={MARK}
                strokeWidth={STROKE}
                aria-hidden
                style={failed?.statusToken ? { color: `var(${failed.statusToken})` } : undefined}
              />
            )}
            <span className="mono">{name}</span>
          </span>
        ))}
      </>
    );
  }
  if (entry.state === "conflict" && entry.conflicts !== undefined) {
    return (
      <>
        {entry.conflicts.map((path) => (
          <span key={path} className="mono">
            {path}
          </span>
        ))}
      </>
    );
  }
  return null;
}
