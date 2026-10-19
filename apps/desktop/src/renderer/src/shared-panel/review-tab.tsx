// A pull request's review tools as a tab of its panel: the explanations, the tools and Ask the Reviews page has, for any pull
// request, whoever owns it. **A tool runs only when pressed**, since an agent run costs, and asks first as it does there.
// The result is the one the Reviews page keeps for the same pull request, so the two always agree.

import { useEffect } from "react";
import { Button, Tooltip } from "@armada/components";
import type { PullReviewRow } from "@armada/protocol";

import { useSendFindings } from "../pr-review/acts";
import { CommentsView } from "../pr-review/comments";
import { AskComposer } from "../pr-review/explain";
import { SpendConfirm } from "../pr-review/flight";
import { Overview } from "../pr-review/reader";
import { ToolResults } from "../pr-review/results";
import { NO_WORK, useReviewStore, useReviews } from "../pr-review/store";
import { onlyTools, toolsOf } from "../pr-review/tools";
import type { ToolDef } from "../pr-review/model";

const typing = (target: EventTarget | null): boolean => target instanceof HTMLElement && (target.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName));

export function ReviewTab({ row, onOpenLink }: { row: PullReviewRow; onOpenLink: (address: string) => void }) {
  const store = useReviewStore();
  const reviews = useReviews();
  const key = `${row.repository}#${row.number}`;
  useEffect(() => {
    if (store === undefined) return;
    store.open(store.adopt(row));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [store, key]);
  const pr = reviews.prs.find((one) => one.key === key);
  const send = useSendFindings(pr);
  const work = pr === undefined ? NO_WORK : (reviews.work[key] ?? NO_WORK);
  // Walk plays the change where it runs and has sheets of its own; it stays on the Reviews page.
  const tools = onlyTools(toolsOf(reviews, pr?.manifestId)).filter((one) => one.mark !== "walk");

  const press = (tool: ToolDef) => {
    if (store === undefined || pr === undefined) return;
    const kept = work.runs[tool.id];
    if (!tool.agent || (kept !== undefined && kept.state !== "idle")) store.runTool(pr.key, tool);
    else store.spend(tool.name, () => store.runTool(pr.key, tool));
  };

  // The keys 1 to 9 press the tools, where no box is being typed in and nothing asks first.
  useEffect(() => {
    const down = (event: KeyboardEvent) => {
      if (event.metaKey || event.ctrlKey || event.altKey || event.shiftKey || typing(event.target) || reviews.spending !== undefined) return;
      const at = Number(event.key) - 1;
      if (!Number.isInteger(at) || at < 0 || at >= tools.length) return;
      event.preventDefault();
      press(tools[at]!);
    };
    window.addEventListener("keydown", down);
    return () => window.removeEventListener("keydown", down);
  });

  if (store === undefined || pr === undefined) return null;
  const tool = tools.find((one) => one.id === work.view);
  return (
    <div className="side-review" aria-label="Review">
      <div className="side-review__tools" role="group" aria-label="Review tools">
        {tools.map((one, at) => (
          <Tooltip key={one.id} label={one.checks} shortcut={String(at + 1)} asChild>
            <Button variant={work.view === one.id ? "secondary" : "ghost"} size="sm" aria-pressed={work.view === one.id} aria-busy={work.runs[one.id]?.state === "running" || undefined} onClick={() => press(one)}>
              {one.name}
            </Button>
          </Tooltip>
        ))}
      </div>
      {work.view === "comments" ? <CommentsView pr={pr} send={send} /> : null}
      {tool !== undefined && tool.agent ? <ToolResults pr={pr} tool={tool} send={send} /> : null}
      {work.view === "explanation" ? <Overview pr={pr} send={send} onOpenLink={onOpenLink} /> : null}
      <AskComposer pr={pr} />
      <SpendConfirm />
    </div>
  );
}
