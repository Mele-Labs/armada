// The line under a pull request's head when it has moved since he last looked: the two commits, how many came between, a Re-run for
// every result kept on the older commit, and a close that marks the new one seen. **The commit he last looked at is kept per pull
// request** and moves only on a Re-run or a close, so leaving and coming back finds the line where it was.

import { useEffect } from "react";
import { RefreshCw, X } from "lucide-react";
import { Button, PrMark, Tooltip, useKept } from "@armada/components";

import type { PrWork, ReviewPr, ToolDef } from "./model";
import { NO_WORK, useReviewStore, useReviews, type ReviewStore } from "./store";
import { isWalk, toolsOf } from "./tools";

const short = (sha: string): string => sha.slice(0, 7);

/** The ids of the tools and explanations with a result kept on a commit other than `head`. */
export function stalePicks(work: PrWork, head: string): string[] {
  const old = (runs: PrWork["runs"]) =>
    Object.entries(runs)
      .filter(([, run]) => run.state === "done" && run.head !== head)
      .map(([id]) => id);
  return [
    ...old(work.explains),
    ...old(work.runs),
  ];
}

/** Runs each of them again, as its own Run again does. Pressed by him, so nothing asks first. */
export function rerunStale(
  store: ReviewStore,
  pr: ReviewPr,
  defs: readonly ToolDef[],
  ids: readonly string[],
): void {
  for (const def of defs.filter((one) => ids.includes(one.id)))
    store.spendUnasked(
      () =>
        def.kind === "explanation"
          ? store.explainAgain(pr.key, def)
          : isWalk(def)
            ? store.findPlace(pr.key)
            : store.rerun(pr.key, def),
      false,
    );
}

/** How many commits the page lists after `seen`, or nothing where the page does not list it. */
const newSince = (pr: ReviewPr, seen: string): number | undefined => {
  const at = pr.page.commits.findIndex((one) => one.sha === seen);
  return at < 0 ? undefined : pr.page.commits.length - at - 1;
};

export function Moved({ pr }: { pr: ReviewPr }) {
  const store = useReviewStore()!;
  const reviews = useReviews();
  const work = reviews.work[pr.key] ?? NO_WORK;
  const [seen, setSeen] = useKept<string>(`pr-review:${pr.key}:seen-head`, "");
  const stale = stalePicks(work, pr.head);
  const moved = seen !== "" && seen !== pr.head;
  // Nothing run on an older commit means there is nothing to offer, so the new one is simply seen. Until Fleet has said what is kept, an empty list says nothing.
  const quiet = seen !== pr.head && (seen === "" || (work.runsRead && stale.length === 0));
  useEffect(() => {
    if (quiet && pr.head !== "") setSeen(pr.head);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    quiet,
    pr.head,
  ]);
  if (!moved || stale.length === 0) return null;
  const count = newSince(pr, seen);
  return (
    <div className="pr-moved" role="status" aria-label="Pull request changed">
      <PrMark
        Glyph={RefreshCw}
        said={`Out of date. Looked at on ${short(seen)}, the pull request is at ${short(pr.head)}`}
      />
      <span className="pr-moved__said">
        <span className="mono">{short(seen)}</span> to{" "}
        <span className="mono">{short(pr.head)}</span>
        {count === undefined || count < 1
          ? null
          : `, ${count} new ${count === 1 ? "commit" : "commits"}`}
      </span>
      <Button
        variant="secondary"
        size="sm"
        onClick={() => {
          rerunStale(store, pr, toolsOf(reviews, pr.manifestId), stale);
          setSeen(pr.head);
        }}
      >
        <RefreshCw size={16} strokeWidth={2} aria-hidden />
        Re-run
      </Button>
      <Tooltip label="Mark this commit as seen">
        <Button
          variant="ghost"
          size="sm"
          iconOnly
          aria-label="Mark as seen"
          onClick={() => setSeen(pr.head)}
        >
          <X size={16} strokeWidth={2} aria-hidden />
        </Button>
      </Tooltip>
    </div>
  );
}
