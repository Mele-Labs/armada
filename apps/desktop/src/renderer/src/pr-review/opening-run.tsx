// What opening the Review tab runs: the tools and explanations of the head row that a repository runs by default, and the ask that
// lists them with a box each. **The default is kept per repository on this machine**, under the manifest's id; with none kept it is
// every tool that finds things, and no explanation. A run kept for this commit is never run again by opening.

import { useState } from "react";
import { CircleDashed, CircleX, History, LoaderCircle, RefreshCw } from "lucide-react";
import { Checkbox, Dialog, PrMark, agoOf, readKept, writeKept } from "@armada/components";

import type { AgentRun, PrWork, ReviewPr, ToolDef } from "./model";
import { openWhenDone } from "./panes";
import type { ReviewStore } from "./store";
import { isWalk, tilesOf } from "./tools";

/** The head row's tools and explanations, in the order + lists them. The Walk is left out: it finds a place before it runs. */
export const headTools = (defs: readonly ToolDef[]): readonly ToolDef[] =>
  tilesOf(defs).filter((one) => one.agent && !isWalk(one));

export const runOf = (work: PrWork, tool: ToolDef): AgentRun | undefined =>
  tool.kind === "explanation" ? work.explains[tool.id] : work.runs[tool.id];

/** Nothing kept for this commit: never run, or done on an earlier one. */
export const unread = (run: AgentRun | undefined, head: string): boolean =>
  run === undefined || run.state === "idle" || (run.state === "done" && run.head !== head);

const keepOf = (manifestId: string) => `pr-review:run-on-opening:${manifestId}`;

/** The ids opening Review runs for a repository: what was kept for it, else every tool that finds things. */
export function defaultsOf(manifestId: string, tools: readonly ToolDef[]): readonly string[] {
  const kept = readKept<string[]>(keepOf(manifestId));
  return kept ?? tools.filter((one) => one.kind === "tool").map((one) => one.id);
}

/** Starts one, as its button in the head does once asked. An explanation's tab opens when it is done. */
export function start(store: ReviewStore, pr: ReviewPr, work: PrWork, tool: ToolDef): void {
  const done = runOf(work, tool)?.state === "done";
  if (tool.kind === "explanation") {
    openWhenDone(pr.key, tool);
    return done ? store.explainAgain(pr.key, tool) : store.explain(pr.key, tool);
  }
  if (done) store.rerun(pr.key, tool);
  else store.runTool(pr.key, tool);
}

/** What a run is, as its button in the head says it: the glyph, and the words its tooltip reads. */
export function runState(run: AgentRun | undefined, head: string) {
  if (run === undefined || run.state === "idle")
    return {
      run: "idle",
      Glyph: CircleDashed,
      said: "Not run",
    } as const;
  if (run.state === "running")
    return {
      run: "running",
      Glyph: LoaderCircle,
      said: "Running",
    } as const;
  if (run.state === "failed")
    return {
      run: "failed",
      Glyph: CircleX,
      said: `Failed. ${run.said}`,
    } as const;
  const stale = run.head !== head;
  return {
    run: "done",
    Glyph: stale ? RefreshCw : History,
    said: stale
      ? `Done on ${run.head.slice(0, 7)}. The pull request is at ${head.slice(0, 7)}`
      : `Done ${agoOf(new Date(run.at).toISOString())} on ${run.head.slice(0, 7)}`,
  } as const;
}

/**
 * The ask on opening: every tool and explanation of the head row with a box, ticked from the repository's default, and its state
 * beside it. Run starts the ticked ones with nothing kept for this commit. Use these keeps the ticked set as the repository's
 * default; Don't ask again turns the question off, as it does for every run.
 */
export function OpeningAsk({
  store,
  pr,
  work,
  tools,
  onClose,
}: {
  store: ReviewStore;
  pr: ReviewPr;
  work: PrWork;
  tools: readonly ToolDef[];
  onClose: () => void;
}) {
  const [ticked, setTicked] = useState<readonly string[]>(() => defaultsOf(pr.manifestId, tools));
  const [keep, setKeep] = useState(false);
  const [again, setAgain] = useState(false);
  const due = tools.filter((one) => ticked.includes(one.id) && unread(runOf(work, one), pr.head));
  const toggle = (id: string) =>
    setTicked((was) =>
      was.includes(id)
        ? was.filter((one) => one !== id)
        : [
            ...was,
            id,
          ],
    );
  return (
    <Dialog
      open
      tone="neutral"
      title="Run on opening"
      confirmLabel="Run"
      confirmDisabled={due.length === 0 && !keep}
      initialFocus="confirm"
      field={
        <div className="pr-opening__keep">
          <Checkbox checked={keep} onChange={(event) => setKeep(event.target.checked)}>
            {`Use these for ${pr.repository} from now on`}
          </Checkbox>
          <Checkbox checked={again} onChange={(event) => setAgain(event.target.checked)}>
            Don't ask again
          </Checkbox>
        </div>
      }
      onConfirm={() => {
        if (keep)
          writeKept(
            keepOf(pr.manifestId),
            tools.filter((one) => ticked.includes(one.id)).map((one) => one.id),
          );
        onClose();
        store.spendUnasked(() => {
          for (const one of due) start(store, pr, work, one);
        }, again);
      }}
      onCancel={onClose}
    >
      <p>Running these uses tokens.</p>
      <ul className="pr-opening__tools" aria-label="Tools to run">
        {tools.map((one) => {
          const { Glyph, said } = runState(runOf(work, one), pr.head);
          return (
            <li key={one.id} className="pr-opening__tool">
              <Checkbox checked={ticked.includes(one.id)} onChange={() => toggle(one.id)}>
                {one.name}
              </Checkbox>
              <PrMark Glyph={Glyph} said={said} />
            </li>
          );
        })}
      </ul>
    </Dialog>
  );
}
