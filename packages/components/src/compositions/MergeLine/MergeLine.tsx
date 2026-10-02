import { ChevronRight, ChevronUp } from "lucide-react";

import { CHECK_OUTCOME, LAND_STATE } from "../../generated/vocabulary";
import { Separator } from "../../primitives/Separator/Separator";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * The merge line: the branches waiting to land on main through `armada land`,
 * in place order, and the ones that just left it. What `armada land --status`
 * prints, drawn as an Overview panel.
 *
 * **A batch is drawn as one group, not as a "together with" string on every
 * member.** The branches a turn gates at once are consecutive in place order,
 * so they nest in one list with a bracket down their leading edge.
 *
 * **State is the mark alone**, from `land_state` in the generated vocabulary,
 * pulsing while the turn holds the branch. Its word is the tooltip. No count
 * is drawn beside the rows, and a slot with nothing to say draws nothing.
 */
export type MergeLineState = "waiting" | "gating" | "merging" | "landed" | "red" | "conflict" | "stopped";

export type MergeLineEntry = {
  branch: string;
  /** 1-based place in line. Absent once the branch has left the line. */
  place?: number;
  /** The open pull request, where the branch has one. */
  pr?: { number: number; url: string };
  state: MergeLineState;
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
};

export type MergeLineProps = {
  /** In place order. */
  line: readonly MergeLineEntry[];
  /** Off the line with an outcome, newest first. */
  off?: readonly MergeLineEntry[];
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Hands the pull request's address to whatever opens addresses on this machine. */
  onOpenPullRequest: (url: string) => void;
  /** On the outer `<section>`, so a press elsewhere can scroll to it. */
  id?: string;
};

const HEADING = "Merge line";

/** The states the runner is working in. Their mark pulses. */
const LIVE: ReadonlySet<MergeLineState> = new Set(["gating", "merging"]);

/** 12px at strokeWidth 2, the badge's own geometry. */
const MARK = 12;
const STROKE = 2;

export function MergeLine({ line, off = [], open, onOpenChange, onOpenPullRequest, id }: MergeLineProps) {
  return (
    <section className="armada-merge-line" id={id} aria-label={HEADING}>
      <header className="armada-merge-line__head">
        <h2 className="armada-merge-line__heading">{HEADING}</h2>
        <button
          type="button"
          className="armada-merge-line__fold"
          aria-expanded={open}
          aria-label={`${open ? "Collapse" : "Expand"} ${HEADING}`}
          onClick={() => onOpenChange(!open)}
        >
          {open ? (
            <ChevronUp size={14} strokeWidth={2} aria-hidden />
          ) : (
            <ChevronRight size={14} strokeWidth={2} aria-hidden />
          )}
        </button>
      </header>
      {open ? (
        <div className="armada-merge-line__body">
          {line.length === 0 ? null : (
            <ol className="armada-merge-line__list" aria-label="In line">
              {batched(line).map((run) =>
                run.length === 1 ? (
                  <Entry key={run[0]!.branch} entry={run[0]!} onOpenPullRequest={onOpenPullRequest} />
                ) : (
                  <li key={run[0]!.branch} className="armada-merge-line__batch">
                    <Tooltip label="Batch" decorative asChild>
                      <span className="armada-merge-line__bracket" aria-hidden />
                    </Tooltip>
                    <ol className="armada-merge-line__list" aria-label="Batch">
                      {run.map((entry) => (
                        <Entry key={entry.branch} entry={entry} onOpenPullRequest={onOpenPullRequest} />
                      ))}
                    </ol>
                  </li>
                ),
              )}
            </ol>
          )}
          {line.length > 0 && off.length > 0 ? <Separator className="armada-merge-line__rule" /> : null}
          {off.length === 0 ? null : (
            <ul className="armada-merge-line__list" aria-label="Left the line">
              {off.map((entry) => (
                <Entry key={entry.branch} entry={entry} onOpenPullRequest={onOpenPullRequest} />
              ))}
            </ul>
          )}
        </div>
      ) : null}
    </section>
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
}: {
  entry: MergeLineEntry;
  onOpenPullRequest: (url: string) => void;
}) {
  const reading = LAND_STATE[entry.state];
  const Icon = reading?.icon ?? null;
  const said = reading?.verb ?? entry.state;
  return (
    <li className="armada-merge-line__row" aria-label={`${entry.branch}, ${said}`}>
      {entry.place === undefined ? (
        <span className="armada-merge-line__place" />
      ) : (
        <Tooltip label="Place in line" asChild>
          <span className="armada-merge-line__place mono">{entry.place}</span>
        </Tooltip>
      )}
      <Tooltip label={said} asChild>
        <span
          className="armada-merge-line__mark"
          data-pulsing={LIVE.has(entry.state) || undefined}
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
      </span>
      <span className="armada-merge-line__detail">
        <Detail entry={entry} />
      </span>
    </li>
  );
}

/** What the row says after its name: the runner's words in a turn, the facts once off the line. */
function Detail({ entry }: { entry: MergeLineEntry }) {
  if (LIVE.has(entry.state)) return entry.doing === undefined ? null : <>{entry.doing}</>;
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
