import { Fragment, useEffect, useRef, useState, type CSSProperties, type FocusEvent } from "react";
import { ArrowUpToLine, Ban, Box, SquareTerminal, ChevronRight, ChevronUp, GitBranch, GitCommitHorizontal, type LucideIcon } from "lucide-react";

import { CHECK_OUTCOME, LAND_STATE, QUEUED_REASON } from "../../generated/vocabulary";
import { Alert, type AlertTone } from "../../primitives/Alert/Alert";
import { Badge } from "../../primitives/Badge/Badge";
import { Button } from "../../primitives/Button/Button";
import { Separator } from "../../primitives/Separator/Separator";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { GUIDE_MERGE_LINE } from "../../guides";
import { OwnerChip, useChipOwner, type OwnerChipRef } from "../OwnerChip/OwnerChip";
import { GuideMark } from "../GuideMark/GuideMark";
import { GroupBoundary, type GroupBoundaryCheck, type GroupBoundaryCheckReads } from "../GroupBoundary/GroupBoundary";
import {
  MainMark,
  MainRedBand,
  type FixChoice,
  type HubJob,
  type MainState,
  type RecentJob,
} from "../MainHead/MainHead";

/**
 * The merge line: the branches waiting to land on main through `armada land`,
 * in place order, then Recently landed and Sent back, drawn as the cockpit's viewscreen: a horizon
 * of dots running into main's light, and each branch a focusable tile in its state's hue.
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
/**
 * `preparing` and `held` are Bridge's own: a `gating` turn that has run no Check yet, and a turn
 * whose failed Check is red on main too.
 */
export type MergeLineState =
  | "waiting"
  | "preparing"
  | "held"
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
  /** The Job the branch came from, where one is known. A Session that owns it is found through `ChipOwnership`. */
  job?: { id: string; title: string };
  /** Waiting only: why it is not in the running turn. Absent draws nothing. */
  why?: MergeLineWaiting;
  /** What the runner is doing to it now, in the runner's own words. Read while gating or merging. */
  doing?: string;
  /** The turn's batch, by key. Consecutive entries with one key gate together. */
  batch?: string;
  /** Landed: the merge commit, short. */
  merge?: string;
  /**
   * Landed: the CI run on the merge commit, on main itself, apart from the pull request's own
   * checks. Absent where nothing ran. `branch` is the Check log's address for its failed jobs.
   */
  mainRun?: { state: "passed" | "running" | "failed"; failed?: readonly string[]; branch: string };
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
  /** The branch whose turn wrote the Check's log. */
  branch: string;
  /**
   * Every branch the Check failed for: a `batch` heads-up lists the turn's members. Absent is the
   * one `branch`.
   */
  branches?: readonly string[];
};

/**
 * An open pull request and how its `ci` stands. `waiting_on_main` is Bridge's own: a `ci` that is red
 * only because main is, so it is the fix's to wait for and not the branch's failure.
 */
export type HubPull = {
  number: number;
  url: string;
  branch: string;
  /** Absent where nothing has run on it: no mark. */
  ci?: "passed" | "running" | "failed" | "waiting_on_main";
  /** The Job it came from. Absent for a person's. */
  job?: HubJob;
  /** Where the merge queue holds it. Absent where it is not in the queue: the `ci` mark stands. */
  queue?: { state: "waiting_for_ci" | "in_queue" | "queued" | "awaiting_checks" | "mergeable" | "unmergeable"; position?: number };
};

/** What the hub adds to the line: main's state, every open pull request, and the Jobs work can go back to. */
export type MergeLineHub = {
  /** Absent where Fleet has not read main, or main is running or ran nothing: no mark and no band. */
  main?: MainState;
  pulls: readonly HubPull[];
  recent: readonly RecentJob[];
};

export type MergeLineProps = {
  /** Main's state and the open pull requests. Absent draws the line alone. */
  hub?: MergeLineHub;
  onOpenJob?: (jobId: string) => void;
  /** Told the owner's choice of a Job for red main. */
  onFix?: (choice: FixChoice) => void;
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
  /** A branch a press elsewhere asked for: its row is marked and scrolled into view. */
  focus?: string;
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
  const { name, line, notice, hub, open, onOpenChange, onOpenPullRequest, onOpenCheck, onOpenJob, onFix, id, focus } = props;
  const acts = {
    onOpenPullRequest,
    ...(onOpenCheck === undefined ? {} : { onOpenCheck }),
    ...(onOpenJob === undefined ? {} : { onOpenJob }),
    ...(focus === undefined ? {} : { focus }),
  };
  const named = name === undefined ? HEADING : `${HEADING}, ${name}`;
  const left = LEFT.map((one) => ({ heading: one.heading, entries: one.pick(props) })).filter(
    (one) => one.entries.length > 0,
  );
  // The tile under the cursor, so its dot on the horizon lights with it.
  const [on, setOn] = useState<string>();
  const lane = notice !== undefined || line.length > 0 || (hub?.pulls.length ?? 0) > 0;
  return (
    <section
      className="armada-merge-line"
      id={id}
      aria-label={named}
      data-red={hub?.main?.state === "red" || undefined}
      onFocus={(event: FocusEvent) => setOn((event.target as HTMLElement).closest<HTMLElement>("[data-branch]")?.dataset.branch)}
      onBlur={() => setOn(undefined)}
    >
      <header className="armada-merge-line__head">
        <div className="armada-merge-line__title">
          <h2 className="armada-merge-line__heading">
            {HEADING}
            {name === undefined ? null : (
              <Tooltip label="Repository" asChild>
                <span className="armada-merge-line__repository">{name}</span>
              </Tooltip>
            )}
          </h2>
          <GuideMark guide={GUIDE_MERGE_LINE} />
          {hub?.main === undefined ? null : <MainMark main={hub.main} />}
        </div>
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
      {hub?.main?.state !== "red" ? null : (
        <MainRedBand
          main={hub.main}
          recent={hub.recent}
          onOpenLink={onOpenPullRequest}
          {...(onOpenCheck === undefined ? {} : { onOpenCheck: (check: string) => onOpenCheck("main", check) })}
          {...(onOpenJob === undefined ? {} : { onOpenJob })}
          {...(onFix === undefined ? {} : { onFix })}
        />
      )}
      {!open ? null : line.length === 0 && left.length === 0 && notice === undefined && (hub?.pulls.length ?? 0) === 0 ? (
        <div className="armada-merge-line__empty">
          <EmptyLine />
        </div>
      ) : (
        <>
          {line.length === 0 && hub?.main === undefined ? null : <Horizon line={line} main={hub?.main} on={on ?? focus} />}
          <div className="armada-merge-line__body">
            {!lane ? null : (
              <div className="armada-merge-line__lane">
                {notice === undefined ? null : (
                  <Notice notice={notice} {...(name === undefined ? {} : { repository: name })} {...(onOpenCheck === undefined ? {} : { onOpenCheck })} />
                )}
                {line.length === 0 ? null : (
                  <>
                    <h3 className="armada-merge-line__subheading">In line</h3>
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
                  </>
                )}
                {hub === undefined || hub.pulls.length === 0 ? null : (
                  <>
                    {line.length > 0 ? <Separator className="armada-merge-line__rule" /> : null}
                    <h3 className="armada-merge-line__subheading">Open pull requests</h3>
                    <ul className="armada-merge-line__list" aria-label="Open pull requests">
                      {hub.pulls.map((pull) => (
                        <Pull key={pull.number} pull={pull} fixing={hub.main?.state === "red" ? hub.main.taken : undefined} onOpenPullRequest={onOpenPullRequest} {...(onOpenJob === undefined ? {} : { onOpenJob })} />
                      ))}
                    </ul>
                  </>
                )}
              </div>
            )}
            {left.length === 0 ? null : (
              <div className="armada-merge-line__side">
                {left.map((one, n) => (
                  <Fragment key={one.heading}>
                    {n > 0 ? <Separator className="armada-merge-line__rule" /> : null}
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
          </div>
        </>
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
  repository,
  onOpenCheck,
}: {
  notice: MergeLineNotice;
  /** The repository, where the panel is one of several. */
  repository?: string;
  onOpenCheck?: (branch: string, check: string) => void;
}) {
  const failed = CHECK_OUTCOME.failed;
  const Shield = failed?.icon ?? null;
  // The row's own mark for the same state, so the two read as one.
  const Held = LAND_STATE.held?.icon ?? null;
  const glyph =
    notice.kind === "main" && Held !== null ? (
      <Tooltip label="Held" asChild>
        <span role="img" aria-label="Held">
          <Held size={16} strokeWidth={2} aria-hidden />
        </span>
      </Tooltip>
    ) : notice.kind === "batch" || Shield === null ? undefined : (
      <Tooltip label="Check failed" asChild>
        <span role="img" aria-label="Check failed">
          <Shield size={16} strokeWidth={2} aria-hidden />
        </span>
      </Tooltip>
    );
  const branches = notice.branches ?? [notice.branch];
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
                <Button
                  variant="secondary"
                  size="sm"
                  ground="sunken"
                  onClick={() => onOpenCheck(notice.branch, notice.check)}
                >
                  Open log
                </Button>
              ),
              actionOn: "title" as const,
            })}
      >
        <span className="armada-merge-line__failed">
          {repository === undefined ? null : (
            <Tooltip label="Repository" asChild>
              <span>{repository}</span>
            </Tooltip>
          )}
          {branches.map((one) => (
            <Tooltip key={one} label="Branch" asChild>
              <span className="mono">{one}</span>
            </Tooltip>
          ))}
        </span>
        <span className="armada-merge-line__facts">
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

/** The states a branch is in the turn in, or held in it: a solid dot. Waiting is a ring. */
const IN_TURN: ReadonlySet<MergeLineState> = new Set(["preparing", "gating", "merging", "held"]);

/** Off the line and wanting the owner: the tile is lit, as a call on the cockpit's glass is. */
const LIT: ReadonlySet<MergeLineState> = new Set(["red", "conflict"]);

/** A row's frame in its state's own status token, never one chosen here. */
function hueOf(token: string | null | undefined): CSSProperties | undefined {
  return token ? ({ "--hue": `var(${token})` } as CSSProperties) : undefined;
}

/**
 * The cockpit's horizon, the width of the page: the line's branches as dots on a belt, moving toward
 * main's light at the right end, place 1 nearest it. A picture of the rows under it, so it is hidden
 * from a screen reader and takes no focus; the tile under the cursor lights its dot.
 */
function Horizon({ line, main, on }: { line: readonly MergeLineEntry[]; main: MainState | undefined; on: string | undefined }) {
  return (
    <div className="armada-merge-line__horizon" aria-hidden>
      <ol className="armada-merge-line__belt">
        {line.map((entry) => (
          <li
            key={entry.branch}
            className="armada-merge-line__dot"
            data-state={entry.state}
            data-queued={IN_TURN.has(entry.state) || undefined}
            data-on={entry.branch === on || undefined}
            style={hueOf(LAND_STATE[entry.state]?.statusToken)}
          />
        ))}
      </ol>
      <span className="armada-merge-line__main" data-state={main?.state} />
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
  onOpenJob,
  focus,
}: {
  entry: MergeLineEntry;
  onOpenPullRequest: (url: string) => void;
  onOpenCheck?: (branch: string, check: string) => void;
  onOpenJob?: (jobId: string) => void;
  focus?: string;
}) {
  const row = useRef<HTMLLIElement>(null);
  const focused = focus === entry.branch;
  useEffect(() => {
    if (focused) row.current?.scrollIntoView?.({ block: "center" });
  }, [focused]);
  const reading = LAND_STATE[entry.state];
  const Icon = reading?.icon ?? null;
  const said = reading?.verb ?? entry.state;
  return (
    <li
      ref={row}
      className="armada-merge-line__row"
      aria-label={`${entry.branch}, ${said}`}
      aria-current={focused ? "true" : undefined}
      tabIndex={0}
      data-merge-item=""
      data-branch={entry.branch}
      data-job={entry.job?.id}
      data-state={entry.state}
      data-lit={LIT.has(entry.state) || undefined}
      style={hueOf(reading?.statusToken)}
    >
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
        <EntryOwner entry={entry} {...(onOpenJob === undefined ? {} : { onOpenJob })} />
      </span>
      <span className="armada-merge-line__detail">
        <Detail entry={entry} {...(onOpenCheck === undefined ? {} : { onOpenCheck })} />
      </span>
    </li>
  );
}

/**
 * Who the entry came from: the Session that owns its pull request or branch, drawn as the owner
 * chip draws it everywhere (hover for the card, press to keep it), else the Job whose branch it
 * is. Nobody known draws nothing.
 */
function EntryOwner({ entry, onOpenJob }: { entry: Pick<MergeLineEntry, "branch" | "pr" | "job">; onOpenJob?: (jobId: string) => void }) {
  const branch: OwnerChipRef = { kind: "branch", name: entry.branch };
  const pull: OwnerChipRef | undefined = entry.pr === undefined ? undefined : { kind: "pull_request", number: entry.pr.number };
  const byBranch = useChipOwner(branch);
  const byPull = useChipOwner(pull ?? branch);
  const session = pull === undefined ? byBranch : (byPull ?? byBranch);
  if (session !== undefined) {
    return (
      <OwnerChip chip={pull !== undefined && byPull !== undefined ? pull : branch}>
        <span className="armada-merge-line__owner">
          <SquareTerminal size={MARK} strokeWidth={STROKE} aria-hidden />
          <span className="armada-merge-line__owner-name">{session.title ?? session.address ?? session.id}</span>
        </span>
      </OwnerChip>
    );
  }
  const job = entry.job;
  if (job === undefined) return null;
  const said = (
    <>
      <Box size={MARK} strokeWidth={STROKE} aria-hidden />
      <span className="armada-merge-line__owner-name">{job.title}</span>
    </>
  );
  return (
    <Tooltip label="Job">
      {onOpenJob === undefined ? (
        <span className="armada-merge-line__owner">{said}</span>
      ) : (
        <button type="button" className="armada-merge-line__owner armada-merge-line__owner--press" onClick={() => onOpenJob(job.id)}>
          {said}
        </button>
      )}
    </Tooltip>
  );
}

/** The `ci` marks on an open pull request, from the Check outcomes and the queue's own reason for waiting. */
const CI: Record<NonNullable<HubPull["ci"]>, { reading: { icon: LucideIcon | null; statusToken: string | null } | undefined; says: string }> = {
  passed: { reading: CHECK_OUTCOME.passed, says: "ci passed" },
  running: { reading: LAND_STATE.gating, says: "ci running" },
  failed: { reading: CHECK_OUTCOME.failed, says: "ci failed" },
  waiting_on_main: { reading: QUEUED_REASON.blocked_by_dependency, says: "ci red because main is" },
};

/**
 * The merge queue's marks on an open pull request, which stand in for its `ci` mark once it has
 * one: a pull request in the queue has passed `ci`. `waiting_for_ci` is auto-merge on and `ci` still
 * running, before any entry.
 */
const QUEUE: Record<NonNullable<HubPull["queue"]>["state"], { reading: { icon: LucideIcon | null; statusToken: string | null } | undefined; says: string; pulsing: boolean }> = {
  waiting_for_ci: { reading: LAND_STATE.preparing, says: "Waiting for ci to join the merge queue", pulsing: true },
  in_queue: { reading: LAND_STATE.waiting, says: "In the merge queue", pulsing: false },
  queued: { reading: LAND_STATE.waiting, says: "In the merge queue, waiting its turn", pulsing: false },
  awaiting_checks: { reading: LAND_STATE.gating, says: "In the merge queue, running its checks", pulsing: true },
  mergeable: { reading: LAND_STATE.merging, says: "In the merge queue, ready to merge", pulsing: true },
  unmergeable: { reading: LAND_STATE.conflict, says: "In the merge queue, cannot merge", pulsing: false },
};

/**
 * The CI run on a landed pull request's merge commit, on main, in the marks an open pull request's
 * `ci` takes. **Apart from the pull request's own checks**, which passed before it merged. A failed
 * run opens its first failed job's log, as main's band does.
 */
function MainRunMark({ run, onOpen }: { run: NonNullable<MergeLineEntry["mainRun"]>; onOpen?: (check: string) => void }) {
  const mark = CI[run.state];
  const Icon = mark.reading?.icon ?? null;
  const said = `${mark.says} on main`;
  const first = run.failed?.[0];
  const glyph = (
    <span
      className="armada-merge-line__mark"
      data-pulsing={run.state === "running" || undefined}
      data-state={run.state === "running" ? "gating" : undefined}
      style={mark.reading?.statusToken ? { color: `var(${mark.reading.statusToken})` } : undefined}
      role="img"
      aria-label={said}
    >
      {Icon === null ? null : <Icon size={MARK} strokeWidth={STROKE} aria-hidden />}
    </span>
  );
  if (run.state !== "failed" || first === undefined || onOpen === undefined) {
    return (
      <Tooltip label={said} asChild>
        {glyph}
      </Tooltip>
    );
  }
  return (
    <Tooltip label={said} asChild>
      <button type="button" className="armada-merge-line__run" aria-label={`${said}, open its log`} onClick={() => onOpen(first)}>
        {glyph}
      </button>
    </Tooltip>
  );
}

/** One open pull request: its `ci` as a mark, the branch, the number, and the Job it came from. */
function Pull({
  pull,
  fixing,
  onOpenPullRequest,
  onOpenJob,
}: {
  pull: HubPull;
  /** The Job working on main's red, named on a pull request that waits for it. */
  fixing: HubJob | undefined;
  onOpenPullRequest: (url: string) => void;
  onOpenJob?: (jobId: string) => void;
}) {
  const queued = pull.queue === undefined ? undefined : QUEUE[pull.queue.state];
  const mark = queued ?? (pull.ci === undefined ? undefined : CI[pull.ci]);
  const Icon = mark?.reading?.icon ?? null;
  const pulsing = queued === undefined ? pull.ci === "running" : queued.pulsing;
  const state = queued === undefined ? (pull.ci === "running" ? "gating" : undefined) : pull.queue?.state === "waiting_for_ci" ? "preparing" : queued.pulsing ? "gating" : undefined;
  const said =
    mark === undefined ? undefined : queued === undefined && pull.ci === "waiting_on_main" && fixing !== undefined ? `${mark.says}, waiting on the fix: ${fixing.title}` : mark.says;
  const position = pull.queue?.position;
  return (
    <li
      className="armada-merge-line__row"
      aria-label={said === undefined ? pull.branch : `${pull.branch}, ${said}`}
      tabIndex={0}
      data-merge-item=""
      data-branch={pull.branch}
      data-job={pull.job?.id}
      data-lit={pull.queue?.state === "unmergeable" || (queued === undefined && pull.ci === "failed") || undefined}
      style={hueOf(mark?.reading?.statusToken)}
    >
      {position === undefined ? (
        <span className="armada-merge-line__place" />
      ) : (
        <Tooltip label="Place in the merge queue" asChild>
          <span className="armada-merge-line__place mono">#{position}</span>
        </Tooltip>
      )}
      {said === undefined ? (
        <span className="armada-merge-line__mark" />
      ) : (
        <Tooltip label={said} asChild>
          <span
            className="armada-merge-line__mark"
            data-pulsing={pulsing || undefined}
            data-state={state}
            style={mark?.reading?.statusToken ? { color: `var(${mark.reading.statusToken})` } : undefined}
            role="img"
            aria-label={said}
          >
            {Icon === null ? null : <Icon size={MARK} strokeWidth={STROKE} aria-hidden />}
          </span>
        </Tooltip>
      )}
      <span className="armada-merge-line__branch mono">{pull.branch}</span>
      <span className="armada-merge-line__pr">
        <Tooltip label="Pull request">
          <a
            href={pull.url}
            onClick={(event) => {
              event.preventDefault();
              onOpenPullRequest(pull.url);
            }}
          >
            #{pull.number}
          </a>
        </Tooltip>
      </span>
      <span className="armada-merge-line__detail">
        <EntryOwner
          entry={{ branch: pull.branch, pr: { number: pull.number, url: pull.url }, ...(pull.job === undefined ? {} : { job: pull.job }) }}
          {...(onOpenJob === undefined ? {} : { onOpenJob })}
        />
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
  // A held turn stops, and still shows what it was doing and where its Checks stand.
  if (LIVE.has(entry.state) || entry.state === "held") {
    return (
      <>
        {entry.doing === undefined ? null : <span>{entry.doing}</span>}
        {strip}
      </>
    );
  }
  if ((entry.state === "red" || entry.state === "stopped") && strip !== null) return strip;
  if (entry.state === "landed" && (entry.merge !== undefined || entry.mainRun !== undefined)) {
    return (
      <>
        {entry.mainRun === undefined ? null : <MainRunMark run={entry.mainRun} {...(onOpenCheck === undefined ? {} : { onOpen: (check: string) => onOpenCheck(entry.mainRun!.branch, check) })} />}
        {entry.merge === undefined ? null : (
          <Tooltip label="Merge commit">
            <span className="mono">{entry.merge}</span>
          </Tooltip>
        )}
      </>
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
