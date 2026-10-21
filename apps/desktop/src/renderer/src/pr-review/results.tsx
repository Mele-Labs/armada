// What a tool found, in the stage's body: each finding pinned to the theme and the change it is about, under the
// change's own title, with the hunk one press away. Armada's alone, hatched, and never posted. Where a Session of his
// holds the pull request, a finding can be sent to it. A way back to the overview is always at the head.

import { ArrowLeft, LoaderCircle, RefreshCw, RotateCw } from "lucide-react";
import { Button, PrMark, Tooltip } from "@armada/components";

import type { useSendFindings } from "./acts";
import { changesOf, isFile, ownerOf, placeOfFinding } from "./changes";
import { RunFailed, SOURCE_MARK, TOOL_MARK } from "./marks";
import type { ReviewPr, ToolDef } from "./model";
import { NothingFound } from "./nothing-found";
import { NO_WORK, useReviewStore, useReviews } from "./store";
import { isExplained } from "./tools";
import { BareChanges, ThemeBlock } from "./themes";
import { PieceActs } from "../sharing/PieceActs";
import { useRootFor } from "./piece-root";

export function ToolResults({ pr, tool, send }: { pr: ReviewPr; tool: ToolDef; send: ReturnType<typeof useSendFindings> }) {
  const store = useReviewStore()!;
  const work = useReviews().work[pr.key] ?? NO_WORK;
  const run = work.runs[tool.id];
  const running = run?.state === "running";
  const stale = run?.state === "done" && run.head !== pr.head;
  const mine = work.findings.filter((one) => one.tool === tool.id && !one.dismissed);
  const all = changesOf(pr, work);
  const owners = new Set(mine.map((one) => ownerOf(all, one.at.path, placeOfFinding(one))?.id));
  const quiet = (one: { id: string }) => !owners.has(one.id);
  const mark = TOOL_MARK[tool.mark];
  const source = SOURCE_MARK[tool.source];
  // A tool of the person's own can be committed to the repository by a Job of its own, or shared.
  const root = useRootFor(pr.manifestId);
  return (
    <div className="pr-results" aria-label={`${tool.name} results`} role="region" data-tool={tool.mark} data-source={tool.source}>
      <header className="pr-results__head">
        <Tooltip label="Back to the overview">
          <Button variant="ghost" size="sm" iconOnly aria-label="Back to the overview" onClick={() => store.setView(pr.key, "explanation")}>
            <ArrowLeft size={16} strokeWidth={2} aria-hidden />
          </Button>
        </Tooltip>
        <PrMark Glyph={mark.Glyph} said={tool.name} />
        <h3 className="pr-results__title">{tool.name}</h3>
        <PrMark Glyph={source.Glyph} said={source.said} />
        <span className="pr-results__acts">
          <Tooltip label="Run again, for this pull request">
            <Button variant="ghost" size="sm" iconOnly aria-label="Run again" disabled={running} onClick={() => store.spend(tool.name, () => store.rerun(pr.key, tool))}>
              <RotateCw size={16} strokeWidth={2} aria-hidden />
            </Button>
          </Tooltip>
        </span>
      </header>
      {tool.source !== "kit" || root === undefined ? null : (
        <div className="pr-results__piece">
          <PieceActs repository={root} piece={{ kind: "review_tool", id: tool.id }} />
        </div>
      )}
      {run?.state === "done" && stale ? (
        <p className="pr-results__stale" role="note" aria-label="Ran on an older commit">
          <PrMark Glyph={RefreshCw} said="Out of date" />
          {`Ran on ${run.head.slice(0, 7)}. The pull request is at ${pr.head.slice(0, 7)}.`}
        </p>
      ) : null}
      {running ? (
        <p className="pr-results__running">
          <span className="pr-review__mark" data-checks="running">
            <PrMark Glyph={LoaderCircle} said="Running" tone="running" />
          </span>
        </p>
      ) : run?.state === "failed" ? (
        <RunFailed said={run.said} />
      ) : mine.length === 0 ? (
        <NothingFound mark={tool.mark} changes={all.length} />
      ) : (
        <div className="pr-results__themes">
          {isExplained(work)
            ? pr.explanation.themes.map((theme, at) => <ThemeBlock key={theme.id} pr={pr} theme={theme} number={at + 1} mode="result" tool={tool.id} send={send} quiet={quiet} />)
            : null}
          <BareChanges pr={pr} mode="result" tool={tool.id} send={send} quiet={(one) => !isFile(one) || quiet(one)} />
        </div>
      )}
    </div>
  );
}
